use std::{io::Write as _, path::PathBuf};

use crate::{
    app::{AppEvent, State},
    utils::event_queue::EventQueue,
};

/// Settings for a scripted benchmark run.
#[derive(Default, clap::Args)]
pub struct BenchmarkSettings {
    /// Record frame times to this CSV file, then exit.
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
    memory_before: Option<u64>,
}

enum Phase {
    WaitingForProject { waited: instant::Duration },
    Warmup { remaining: usize },
    Recording { frames: Vec<f32> },
    Done,
}

impl Benchmark {
    pub fn new(settings: BenchmarkSettings, adapter_info: &wgpu::AdapterInfo) -> Option<Self> {
        Some(Self {
            out: settings.out?,
            total_frames: settings.benchmark_frames,
            warmup_frames: settings.benchmark_warmup,
            adapter_info: adapter_info.clone(),
            phase: Phase::WaitingForProject {
                waited: instant::Duration::ZERO,
            },
            memory_before: None,
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
                if matches!(state, State::Workspace(workspace) if !workspace.is_rebuilding()) {
                    log::info!(
                        "discarding {} frames, then recording {}",
                        self.warmup_frames,
                        self.total_frames
                    );
                    self.memory_before = memory_bytes();
                    self.phase = Phase::Warmup {
                        remaining: self.warmup_frames,
                    };
                } else {
                    *waited += dt;
                    if *waited >= PROJECT_TIMEOUT {
                        let timeout = PROJECT_TIMEOUT.as_secs();
                        log::error!("the project was still not ready after {timeout}s, giving up",);
                        self.phase = Phase::Done;
                        events.quit();
                    }
                }
            }
            Phase::Warmup { remaining } => {
                *remaining = remaining.saturating_sub(1);
                if *remaining == 0 {
                    self.phase = Phase::Recording {
                        frames: Vec::with_capacity(self.total_frames),
                    };
                }
            }
            Phase::Recording { frames } => {
                frames.push(dt.as_secs_f32() * 1000.0);

                if frames.len() >= self.total_frames {
                    let frames = std::mem::take(frames);
                    self.phase = Phase::Done;
                    self.finish(&frames, state, present_mode);
                    events.quit();
                }
            }
            Phase::Done => {}
        }
    }

    fn finish(&self, frames: &[f32], state: &State, present_mode: wgpu::PresentMode) {
        let memory = memory_bytes();

        let mean = frames.iter().sum::<f32>() / frames.len() as f32;
        let frames_count = frames.len();
        let average_fps = 1000.0 / mean;
        log::info!("{frames_count} frames, {mean:.3} ms on average ({average_fps:.1} FPS)",);

        match self.write_csv(frames, state, present_mode, memory) {
            Ok(()) => log::info!("wrote {}", self.out.display()),
            Err(error) => log::error!("failed to write {}: {error}", self.out.display()),
        }
    }

    fn write_csv(
        &self,
        frames: &[f32],
        state: &State,
        present_mode: wgpu::PresentMode,
        memory: Option<u64>,
    ) -> std::io::Result<()> {
        use crate::built_info;

        let project = match state {
            State::Workspace(workspace) => workspace.project_name(),
            State::MainMenu(_) => "none",
        };

        let mut file = std::io::BufWriter::new(std::fs::File::create(&self.out)?);

        let wgpu::AdapterInfo {
            backend,
            name: gpu_name,
            driver_info,
            ..
        } = &self.adapter_info;

        let version = built_info::PKG_VERSION;
        let commit = built_info::GIT_COMMIT_HASH_SHORT.unwrap_or("unknown commit");

        let memory = format_memory(memory);
        let memory_before = format_memory(self.memory_before);

        writeln!(file, "# rau {version} ({commit})",)?;
        writeln!(file, "# profile: {}", built_info::PROFILE)?;
        writeln!(file, "# project: {project}")?;
        writeln!(file, "# backend: {backend:?}")?;
        writeln!(file, "# gpu: {gpu_name} ({driver_info})",)?;
        writeln!(file, "# cpu: {}", cpu_name())?;
        writeln!(file, "# present_mode: {present_mode:?}")?;
        writeln!(file, "# warmup_frames: {}", self.warmup_frames)?;
        writeln!(file, "# memory: start: {memory_before}, end: {memory}",)?;
        writeln!(file, "frame,frame_ms")?;

        for (index, frame_ms) in frames.iter().enumerate() {
            writeln!(file, "{index},{frame_ms:.4}")?;
        }

        file.flush()
    }
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

fn memory_bytes() -> Option<u64> {
    let pid = sysinfo::get_current_pid().ok()?;

    let mut system = sysinfo::System::new();
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[pid]),
        true,
        sysinfo::ProcessRefreshKind::nothing().with_memory(),
    );

    Some(system.process(pid)?.memory())
}

fn format_memory(bytes: Option<u64>) -> String {
    match bytes {
        Some(bytes) => format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0)),
        None => "unknown".to_owned(),
    }
}
