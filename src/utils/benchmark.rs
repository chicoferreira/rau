use std::{
    io::Write as _,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
};

use crate::app::State;

/// Settings for a scripted benchmark run.
pub struct BenchmarkSettings {
    /// Where to write every `puffin` span of a fixed window of frames.
    pub out: PathBuf,
    /// Where to write the spans of the project loading, if anywhere.
    pub load_out: Option<PathBuf>,
    pub record_duration: instant::Duration,
    pub warmup_duration: instant::Duration,
}

impl BenchmarkSettings {
    pub fn new(
        out: PathBuf,
        load_out: Option<PathBuf>,
        record_duration: Option<instant::Duration>,
        warmup_duration: Option<instant::Duration>,
    ) -> Self {
        Self {
            out,
            load_out,
            record_duration: record_duration.unwrap_or(instant::Duration::from_secs(30)),
            warmup_duration: warmup_duration.unwrap_or(instant::Duration::from_secs(10)),
        }
    }
}

const PROJECT_TIMEOUT: instant::Duration = instant::Duration::from_secs(30);

pub struct Benchmark {
    settings: BenchmarkSettings,
    adapter_info: wgpu::AdapterInfo,
    phase: Phase,
    sink: puffin::FrameSinkId,
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
    pub fn new(settings: BenchmarkSettings, adapter_info: wgpu::AdapterInfo) -> Self {
        puffin::set_scopes_on(true);

        let (sender, recorded) = std::sync::mpsc::channel();
        let sink = record_into(sender);

        Self {
            settings,
            adapter_info,
            phase: Phase::WaitingForProject {
                waited: instant::Duration::ZERO,
            },
            sink,
            recorded,
        }
    }

    pub fn tick(&mut self, dt: instant::Duration, state: &State, present_mode: wgpu::PresentMode) {
        match &mut self.phase {
            Phase::WaitingForProject { waited } => {
                *waited += dt;

                if matches!(state, State::Workspace(workspace) if !workspace.is_rebuilding()) {
                    log::info!(
                        "Recorded the load in {waited:.1?}, then discarding {:.1?} and recording {:.1?}",
                        self.settings.warmup_duration,
                        self.settings.record_duration
                    );

                    self.phase = Phase::Warmup {
                        elapsed: instant::Duration::ZERO,
                    };

                    self.finish(Capture::Load, state, present_mode);
                } else if *waited >= PROJECT_TIMEOUT {
                    let timeout = PROJECT_TIMEOUT.as_secs();
                    log::error!("Project still not ready after {timeout}s, giving up");
                    self.phase = Phase::Done;
                }
            }
            Phase::Warmup { elapsed } => {
                *elapsed += dt;

                if *elapsed >= self.settings.warmup_duration {
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

                if *elapsed >= self.settings.record_duration {
                    self.phase = Phase::Done;
                    self.finish(Capture::Frames, state, present_mode);
                }
            }
            Phase::Done => {}
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self.phase, Phase::Done)
    }

    fn finish(&self, capture: Capture, state: &State, present_mode: wgpu::PresentMode) {
        let out = match capture {
            Capture::Load => match &self.settings.load_out {
                Some(out) => out.clone(),
                None => return,
            },
            Capture::Frames => self.settings.out.clone(),
        };

        let covers = match capture {
            Capture::Load => "the load".to_owned(),
            Capture::Frames => {
                let record_dur = self.settings.record_duration;
                let warmup_dur = self.settings.warmup_duration;
                format!("{record_dur:.1?} of frames after {warmup_dur:.1?} of warm-up")
            }
        };

        let result = self
            .write_csv(&covers, state, present_mode)
            .and_then(|csv| save(&out, csv));

        match result {
            Ok(()) => log::info!("Wrote the spans to {out:?}"),
            Err(error) => log::error!("Failed to write {out:?}: {error}"),
        }
    }

    fn write_csv(
        &self,
        covers: &str,
        state: &State,
        present_mode: wgpu::PresentMode,
    ) -> anyhow::Result<Vec<u8>> {
        use crate::built_info;

        let mut bytes = Vec::new();

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

        writeln!(bytes, "# rau {version} ({commit})")?;
        writeln!(bytes, "# profile: {profile}")?;
        writeln!(bytes, "# project: {project}")?;
        writeln!(bytes, "# backend: {backend:?}")?;
        writeln!(bytes, "# gpu: {gpu_name} ({driver_info})")?;
        writeln!(bytes, "# cpu: {}", cpu_name())?;
        writeln!(bytes, "# present_mode: {present_mode:?}")?;
        writeln!(bytes, "# covers: {covers}")?;

        writeln!(bytes, "frame,thread,depth,scope,data,start_ns,duration_ns")?;

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
                    &mut bytes,
                    &stream.stream,
                    0,
                    0,
                    frame.frame_index(),
                    &thread.name,
                    &scopes,
                )?;
            }
        }

        Ok(bytes)
    }
}

impl Drop for Benchmark {
    fn drop(&mut self) {
        puffin::GlobalProfiler::lock().remove_sink(self.sink);
    }
}

fn record_into(sender: Sender<Arc<puffin::FrameData>>) -> puffin::FrameSinkId {
    let mut profiler = puffin::GlobalProfiler::lock();
    let sink = profiler.add_sink(Box::new(move |frame| {
        let _ = sender.send(frame);
    }));

    profiler.emit_scope_snapshot();
    sink
}

fn save(out: &std::path::Path, csv: Vec<u8>) -> anyhow::Result<()> {
    cfg_select! {
        target_arch = "wasm32" => crate::utils::browser::download_file(&out.to_string_lossy(), csv)
            .map_err(|error| anyhow::anyhow!("{error}")),
        _ => Ok(std::fs::write(out, csv)?),
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
