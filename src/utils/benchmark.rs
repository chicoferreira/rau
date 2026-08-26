use std::{
    io::Write as _,
    path::PathBuf,
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
    /// Record every `puffin` span of a fixed number of frames to this CSV file, then exit.
    #[arg(long = "benchmark", value_name = "FILE", global = true)]
    pub out: Option<PathBuf>,

    /// How many frames to record.
    #[arg(long, global = true, default_value_t = 1000, requires = "out")]
    pub benchmark_frames: usize,

    /// How many frames to discard after the project is ready, letting the frame rate settle.
    #[arg(long, global = true, default_value_t = 200, requires = "out")]
    pub benchmark_warmup: usize,
}

const PROJECT_TIMEOUT: instant::Duration = instant::Duration::from_secs(30);

pub struct Benchmark {
    out: PathBuf,
    total_frames: usize,
    warmup_frames: usize,
    adapter_info: wgpu::AdapterInfo,
    phase: Phase,
    sender: Sender<Arc<puffin::FrameData>>,
    recorded: Receiver<Arc<puffin::FrameData>>,
}

enum Phase {
    WaitingForProject { waited: instant::Duration },
    Warmup { remaining: usize },
    Recording { remaining: usize },
    Done,
}

impl Benchmark {
    pub fn new(settings: BenchmarkSettings, adapter_info: &wgpu::AdapterInfo) -> Option<Self> {
        let out = settings.out?;

        puffin::set_scopes_on(true);

        let (sender, recorded) = std::sync::mpsc::channel();

        Some(Self {
            out,
            total_frames: settings.benchmark_frames,
            warmup_frames: settings.benchmark_warmup,
            adapter_info: adapter_info.clone(),
            phase: Phase::WaitingForProject {
                waited: instant::Duration::ZERO,
            },
            sender,
            recorded,
        })
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
                        "Project ready after {:.1?}, discarding {} frames, then running {}",
                        waited,
                        self.warmup_frames,
                        self.total_frames
                    );
                    self.phase = Phase::Warmup {
                        remaining: self.warmup_frames,
                    };
                } else if *waited >= PROJECT_TIMEOUT {
                    let timeout = PROJECT_TIMEOUT.as_secs();
                    log::error!("Project still not ready after {timeout}s, giving up");
                    self.phase = Phase::Done;
                    events.quit();
                }
            }
            Phase::Warmup { remaining } => {
                *remaining = remaining.saturating_sub(1);
                if *remaining == 0 {
                    self.begin_recording();

                    self.phase = Phase::Recording {
                        remaining: self.total_frames,
                    };
                }
            }
            Phase::Recording { remaining } => {
                *remaining = remaining.saturating_sub(1);

                if *remaining == 0 {
                    self.phase = Phase::Done;
                    self.finish(state, present_mode);
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

    fn finish(&self, state: &State, present_mode: wgpu::PresentMode) {
        match self.write_csv(state, present_mode) {
            Ok(()) => log::info!("Wrote the spans to {:?}", self.out),
            Err(error) => log::error!("Failed to write {:?}: {error}", self.out),
        }
    }

    fn write_csv(&self, state: &State, present_mode: wgpu::PresentMode) -> anyhow::Result<()> {
        use crate::built_info;

        let mut file = std::io::BufWriter::new(std::fs::File::create(&self.out)?);

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

        let frames = self.total_frames;
        let warmup = self.warmup_frames;

        writeln!(file, "# rau {version} ({commit})")?;
        writeln!(file, "# profile: {profile}")?;
        writeln!(file, "# project: {project}")?;
        writeln!(file, "# backend: {backend:?}")?;
        writeln!(file, "# gpu: {gpu_name} ({driver_info})")?;
        writeln!(file, "# cpu: {}", cpu_name())?;
        writeln!(file, "# present_mode: {present_mode:?}")?;
        writeln!(file, "# frames: {frames}, after a warm-up of {warmup}")?;
        writeln!(file, "frame,thread,depth,scope,start_ns,duration_ns")?;

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
            ..
        } = scope.record;

        writeln!(
            file,
            r#"{frame},"{thread}",{depth},"{name}",{start_ns},{duration_ns}"#
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
