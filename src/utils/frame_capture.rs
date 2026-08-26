use std::{io::Write as _, path::PathBuf};

use crate::{
    app::{AppEvent, State},
    utils::event_queue::EventQueue,
};

/// Settings for a scripted frame time capture.
#[derive(Default, clap::Args)]
pub struct CaptureSettings {
    /// Record frame times to this CSV file, then exit.
    #[arg(long = "capture", value_name = "FILE", global = true)]
    pub out: Option<PathBuf>,

    /// How many frames to record.
    #[arg(long, global = true, default_value_t = 1000, requires = "out")]
    pub capture_frames: usize,

    /// How many frames to discard after the project is ready, letting the frame rate settle.
    #[arg(long, global = true, default_value_t = 200, requires = "out")]
    pub capture_warmup: usize,
}

pub struct FrameCapture {
    out: PathBuf,
    total_frames: usize,
    warmup_frames: usize,
    adapter_info: wgpu::AdapterInfo,
    phase: Phase,
}

enum Phase {
    WaitingForProject,
    Warmup { remaining: usize },
    Recording { frames: Vec<f32> },
    Done,
}

impl FrameCapture {
    pub fn new(settings: CaptureSettings, adapter_info: &wgpu::AdapterInfo) -> Option<Self> {
        Some(Self {
            out: settings.out?,
            total_frames: settings.capture_frames,
            warmup_frames: settings.capture_warmup,
            adapter_info: adapter_info.clone(),
            phase: Phase::WaitingForProject,
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
            Phase::WaitingForProject => {
                if matches!(state, State::Workspace(workspace) if !workspace.is_rebuilding()) {
                    log::info!(
                        "Capture: discarding {} frames, then recording {}",
                        self.warmup_frames,
                        self.total_frames
                    );
                    self.phase = Phase::Warmup {
                        remaining: self.warmup_frames,
                    };
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
        let mean = frames.iter().sum::<f32>() / frames.len() as f32;
        log::info!(
            "Capture: {} frames, {mean:.3} ms on average ({:.1} FPS)",
            frames.len(),
            1000.0 / mean
        );

        match self.write_csv(frames, state, present_mode) {
            Ok(()) => log::info!("Capture: wrote {}", self.out.display()),
            Err(error) => log::error!("Capture: failed to write {}: {error}", self.out.display()),
        }
    }

    fn write_csv(
        &self,
        frames: &[f32],
        state: &State,
        present_mode: wgpu::PresentMode,
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

        // Metadata lives in comment lines so a capture stays readable on its own, while every
        // CSV reader still sees just the two columns.
        writeln!(file, "# rau {version} ({commit})",)?;
        writeln!(file, "# profile: {}", built_info::PROFILE)?;
        writeln!(file, "# project: {project}")?;
        writeln!(file, "# backend: {backend:?}")?;
        writeln!(file, "# gpu: {gpu_name} ({driver_info})",)?;
        writeln!(file, "# cpu: {}", cpu_name())?;
        writeln!(file, "# present_mode: {present_mode:?}")?;
        writeln!(file, "# warmup_frames: {}", self.warmup_frames)?;
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
