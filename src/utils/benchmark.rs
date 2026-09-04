use std::{
    io::Write as _,
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
};

use crate::{
    app::{AppEvent, State},
    utils::event_queue::EventQueue,
};

/// Settings for a scripted benchmark run.
#[derive(Default, clap::Args)]
pub struct BenchmarkSettings {
    /// Record every `puffin` span of a fixed window of time to this CSV file, then exit.
    #[arg(long = "benchmark", value_name = "FILE", global = true)]
    pub out: Option<PathBuf>,

    /// How many seconds to record.
    #[arg(
        long,
        global = true,
        value_name = "SECONDS",
        value_parser = seconds,
        default_value = "30",
        requires = "out"
    )]
    pub benchmark_seconds: instant::Duration,

    /// How many seconds to wait after the project is ready for the benchmark to start.
    #[arg(
        long,
        global = true,
        value_name = "SECONDS",
        value_parser = seconds,
        default_value = "10",
        requires = "out"
    )]
    pub benchmark_warmup_seconds: instant::Duration,
}

fn seconds(value: &str) -> Result<instant::Duration, String> {
    let seconds: f64 = value
        .parse()
        .map_err(|_| format!("`{value}` is not a number"))?;

    instant::Duration::try_from_secs_f64(seconds)
        .map_err(|error| format!("`{value}` is not a valid number of seconds: {error}"))
}

const PROJECT_TIMEOUT: instant::Duration = instant::Duration::from_secs(30);

pub struct Benchmark {
    out: PathBuf,
    record_duration: instant::Duration,
    warmup_duration: instant::Duration,
    adapter_info: wgpu::AdapterInfo,
    phase: Phase,
    sender: Sender<Arc<puffin::FrameData>>,
    recorded: Receiver<Arc<puffin::FrameData>>,
}

enum Capture {
    Load,
    Frames,
}

enum Phase {
    WaitingForProject { waited: instant::Duration },
    Warmup { elapsed: instant::Duration },
    Recording { elapsed: instant::Duration },
    Done,
}

impl Benchmark {
    pub fn new(settings: BenchmarkSettings, adapter_info: &wgpu::AdapterInfo) -> Option<Self> {
        let out = settings.out?;

        puffin::set_scopes_on(true);

        let (sender, recorded) = std::sync::mpsc::channel();

        let mut benchmark = Self {
            out,
            record_duration: settings.benchmark_seconds,
            warmup_duration: settings.benchmark_warmup_seconds,
            adapter_info: adapter_info.clone(),
            phase: Phase::WaitingForProject {
                waited: instant::Duration::ZERO,
            },
            sender,
            recorded,
        };

        benchmark.begin_recording();

        Some(benchmark)
    }

    pub fn tick(
        &mut self,
        dt: instant::Duration,
        state: &State,
        present_mode: wgpu::PresentMode,
        events: &mut EventQueue<AppEvent>,
    ) {
        match &mut self.phase {
            Phase::WaitingForProject { waited } => {
                *waited += dt;

                if matches!(state, State::Workspace(workspace) if !workspace.is_rebuilding()) {
                    log::info!(
                        "Recorded the load in {waited:.1?}, then discarding {:.1?} and recording {:.1?}",
                        self.warmup_duration,
                        self.record_duration
                    );

                    self.phase = Phase::Warmup {
                        elapsed: instant::Duration::ZERO,
                    };

                    self.finish(Capture::Load, state, present_mode);
                } else if *waited >= PROJECT_TIMEOUT {
                    let timeout = PROJECT_TIMEOUT.as_secs();
                    log::error!("Project still not ready after {timeout}s, giving up");
                    self.phase = Phase::Done;
                    events.quit();
                }
            }
            Phase::Warmup { elapsed } => {
                *elapsed += dt;

                if *elapsed >= self.warmup_duration {
                    // Clear the recorded frames so the CSV only contains the frames after the warmup.
                    self.recorded.try_iter().for_each(drop);
                    puffin::GlobalProfiler::lock().emit_scope_snapshot();

                    self.phase = Phase::Recording {
                        elapsed: instant::Duration::ZERO,
                    };
                }
            }
            Phase::Recording { elapsed } => {
                *elapsed += dt;

                if *elapsed >= self.record_duration {
                    self.phase = Phase::Done;
                    self.finish(Capture::Frames, state, present_mode);
                    events.quit();
                }
            }
            Phase::Done => {}
        }
    }

    fn begin_recording(&mut self) {
        let sender = self.sender.clone();

        let mut profiler = puffin::GlobalProfiler::lock();
        profiler.add_sink(Box::new(move |frame| {
            let _ = sender.send(frame);
        }));

        profiler.emit_scope_snapshot();
    }

    fn finish(&self, capture: Capture, state: &State, present_mode: wgpu::PresentMode) {
        let out = match capture {
            Capture::Load => self.out.with_extension("loading.csv"),
            Capture::Frames => self.out.clone(),
        };

        let covers = match capture {
            Capture::Load => "the load".to_owned(),
            Capture::Frames => {
                let record_dur = self.record_duration;
                let warmup_dur = self.warmup_duration;
                format!("{record_dur:.1?} of frames after {warmup_dur:.1?} of warm-up")
            }
        };

        match self.write_csv(&out, &covers, state, present_mode) {
            Ok(()) => log::info!("Wrote the spans to {out:?}"),
            Err(error) => log::error!("Failed to write {out:?}: {error}"),
        }
    }

    fn write_csv(
        &self,
        out: &Path,
        covers: &str,
        state: &State,
        present_mode: wgpu::PresentMode,
    ) -> anyhow::Result<()> {
        use crate::built_info;

        let mut file = std::io::BufWriter::new(std::fs::File::create(out)?);

        let project = match state {
            State::Workspace(workspace) => workspace.project_name(),
            State::MainMenu(_) => "none",
        };

        let wgpu::AdapterInfo {
            backend,
            name: gpu_name,
            driver_info,
            ..
        } = &self.adapter_info;

        let version = built_info::PKG_VERSION;
        let profile = built_info::PROFILE;
        let commit = match (built_info::GIT_COMMIT_HASH_SHORT, built_info::GIT_DIRTY) {
            (Some(commit), Some(true)) => format!("{commit}-dirty"),
            (Some(commit), _) => commit.to_owned(),
            (None, _) => "unknown commit".to_owned(),
        };

        writeln!(file, "# rau {version} ({commit})")?;
        writeln!(file, "# profile: {profile}")?;
        writeln!(file, "# project: {project}")?;
        writeln!(file, "# backend: {backend:?}")?;
        writeln!(file, "# gpu: {gpu_name} ({driver_info})")?;
        writeln!(file, "# cpu: {}", cpu_name())?;
        writeln!(file, "# present_mode: {present_mode:?}")?;
        writeln!(file, "# covers: {covers}")?;

        writeln!(file, "frame,thread,depth,scope,data,start_ns,duration_ns")?;

        let mut scopes = puffin::ScopeCollection::default();

        for frame in self.recorded.try_iter() {
            for details in &frame.scope_delta {
                scopes.insert(details.clone());
            }

            let unpacked = frame
                .unpacked()
                .map_err(|error| anyhow::anyhow!("Failed to unpack frame: {error:?}"))?;

            for (thread, stream) in &unpacked.thread_streams {
                write_spans(
                    &mut file,
                    &stream.stream,
                    0,
                    0,
                    frame.frame_index(),
                    &thread.name,
                    &scopes,
                )?;
            }
        }

        Ok(())
    }
}

fn write_spans(
    file: &mut impl std::io::Write,
    stream: &puffin::Stream,
    offset: u64,
    depth: usize,
    frame: u64,
    thread: &str,
    scopes: &puffin::ScopeCollection,
) -> anyhow::Result<()> {
    let reader = puffin::Reader::with_offset(stream, offset)
        .map_err(|error| anyhow::anyhow!("Failed to create puffin reader: {error:?}"))?;

    for scope in reader {
        let scope =
            scope.map_err(|error| anyhow::anyhow!("Failed to read puffin scope: {error:?}"))?;

        let name = scopes
            .fetch_by_id(&scope.id)
            .map_or("unknown", |details| details.name().as_ref());

        let puffin::ScopeRecord {
            start_ns,
            duration_ns,
            data,
        } = scope.record;

        writeln!(
            file,
            r#"{frame},"{thread}",{depth},"{name}","{data}",{start_ns},{duration_ns}"#
        )?;

        write_spans(
            file,
            stream,
            scope.child_begin_position,
            depth + 1,
            frame,
            thread,
            scopes,
        )?;
    }

    Ok(())
}

fn cpu_name() -> String {
    let system = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_cpu(sysinfo::CpuRefreshKind::nothing()),
    );

    system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim())
        .filter(|brand| !brand.is_empty())
        .unwrap_or("unknown")
        .to_owned()
}
