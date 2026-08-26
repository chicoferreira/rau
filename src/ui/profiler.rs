use std::sync::Arc;

use puffin_egui::MaybeMutRef;
use wgpu_profiler::{GpuProfiler, GpuProfilerSettings};

use crate::{error::AppResult, ui::components::hint, utils::wgpu_utils::create_command_encoder};

pub struct Profiler {
    tab: Tab,
    gpu: GpuTimeline,
    gpu_profiler: GpuProfiler,
    gpu_timings_supported: bool,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Tab {
    #[default]
    Cpu,
    Gpu,
}

impl Profiler {
    pub fn new(device: &wgpu::Device) -> AppResult<Self> {
        let gpu_timings_supported = device.features().contains(wgpu::Features::TIMESTAMP_QUERY);
        log::info!("GPU timestamp queries supported: {gpu_timings_supported}");

        Ok(Self {
            tab: Tab::default(),
            gpu: GpuTimeline::default(),
            gpu_profiler: GpuProfiler::new(device, GpuProfilerSettings::default())?,
            gpu_timings_supported,
        })
    }

    pub fn gpu_profiler(&self) -> &GpuProfiler {
        &self.gpu_profiler
    }

    /// Whether the GPU is being timed this frame.
    fn timing_gpu(&self) -> bool {
        self.gpu_profiler.settings().enable_timer_queries
    }

    pub fn begin_frame(&mut self) {
        let should_time_gpu = puffin::are_scopes_on();
        if should_time_gpu == self.timing_gpu() {
            return;
        }

        let settings = GpuProfilerSettings {
            enable_timer_queries: should_time_gpu,
            ..Default::default()
        };

        if let Err(error) = self.gpu_profiler.change_settings(settings) {
            log::error!("Failed to change the GPU profiler settings: {error}");
        }
    }

    pub fn end_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if !self.timing_gpu() {
            return;
        }

        let mut encoder = create_command_encoder(device, "Profiler Resolve Encoder");
        self.gpu_profiler.resolve_queries(&mut encoder);
        queue.submit([encoder.finish()]);

        if let Err(error) = self.gpu_profiler.end_frame() {
            log::error!("Failed to end the GPU profiler frame: {error}");
            return;
        }

        let period = queue.get_timestamp_period();
        if let Some(results) = self.gpu_profiler.process_finished_frame(period) {
            wgpu_profiler::puffin::output_frame_to_puffin(&mut self.gpu.profiler, &results);
            self.gpu.profiler.new_frame();
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        puffin::profile_function!();

        if !puffin::are_scopes_on() {
            return;
        }

        let mut open = true;
        egui::Window::new("Profiler")
            .default_size([1024.0, 600.0])
            .open(&mut open)
            .show(ctx, |ui| self.contents(ui));

        if !open {
            puffin::set_scopes_on(false);
        }
    }

    fn contents(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Cpu, "CPU");
            ui.selectable_value(&mut self.tab, Tab::Gpu, "GPU");
        });

        ui.separator();

        match self.tab {
            Tab::Cpu => {
                if cpu_timings_clamped() {
                    empty_tab_hint(
                        ui,
                        "This page isn't cross-origin isolated, so the flamegraph may not have sufficient detail. Serve this page with the COOP and COEP headers in `web/_headers` to lift that.",
                    );
                }

                puffin_egui::profiler_ui(ui);
            }
            Tab::Gpu => {
                if !self.gpu_timings_supported {
                    empty_tab_hint(ui, "This backend doesn't support timestamp queries.");
                } else if cfg!(target_arch = "wasm32") {
                    empty_tab_hint(
                        ui,
                        "Enable WebGPU developer features flag for more granular timing.",
                    );
                }

                self.gpu.ui(ui);
            }
        }
    }
}

/// Whether the browser is rounding off the clock the profiler measures with.
fn cpu_timings_clamped() -> bool {
    cfg_select! {
        target_arch = "wasm32" => !crate::utils::browser::is_cross_origin_isolated(),
        _ => false,
    }
}

fn empty_tab_hint(ui: &mut egui::Ui, text: &str) {
    ui.add(hint::hint(|ui| {
        ui.label(text);
    }));
    ui.add_space(4.0);
}

struct GpuTimeline {
    profiler: puffin::GlobalProfiler,
    view: Arc<egui::mutex::Mutex<puffin::FrameView>>,
    ui: puffin_egui::ProfilerUi,
}

impl Default for GpuTimeline {
    fn default() -> Self {
        let view = Arc::new(egui::mutex::Mutex::new(puffin::FrameView::default()));

        // Finished frames reach the view through this sink.
        let mut profiler = puffin::GlobalProfiler::default();
        let sink_view = view.clone();
        profiler.add_sink(Box::new(move |frame| sink_view.lock().add_frame(frame)));

        Self {
            profiler,
            view,
            ui: puffin_egui::ProfilerUi::default(),
        }
    }
}

impl GpuTimeline {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut view = self.view.lock();
        self.ui.ui(ui, &mut MaybeMutRef::MutRef(&mut view));
    }
}
