use std::task::Poll;

use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    project::{
        Creatable, ProjectResource, TextureId, TextureViewId,
        resource::texture::{Texture, TextureRuntime},
        storage::{RuntimeStorage, Storage},
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    resource_getters, resource_setters,
    ui::renderer::EguiRenderer,
    utils::{async_job::AsyncJob, wgpu_error_scope::WgpuErrorScope},
};

pub struct TextureViewCreationContext<'a> {
    pub textures: &'a Storage<Texture>,
    pub textures_runtime: &'a RuntimeStorage<Texture>,
    pub egui_renderer: &'a mut EguiRenderer,
    pub device: &'a wgpu::Device,
    pub downlevel_flags: wgpu::DownlevelFlags,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureView {
    label: String,
    format: Option<TextureViewFormat>,
    dimension: Option<wgpu::TextureViewDimension>,
    texture_id: Option<TextureId>,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

pub struct TextureViewRuntime {
    inner: wgpu::TextureView,
    egui_id: Option<egui::TextureId>,
}

#[derive(Default)]
pub enum TextureViewJob {
    #[default]
    Start,
    PendingResource {
        previous_egui_id: Option<egui::TextureId>,
    },
    Validation(TextureViewRuntime, AsyncJob<AppResult<()>>),
}

/// As currently the texture view format is only allowed to change by srgb-ness
/// This will allow the user to easily specify it
///
/// Check [`wgpu::wgt::TextureDescriptor::view_formats`]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TextureViewFormat {
    Srgb,
    Linear,
}

// TODO: change this to a better filter, i think it should work with every RGBA texture format
const ALLOWED_EGUI_FORMATS: &[wgpu::TextureFormat] = &[
    wgpu::TextureFormat::Rgba8UnormSrgb,
    wgpu::TextureFormat::Rgba8Unorm,
    wgpu::TextureFormat::Rgba16Float,
];

impl TextureView {
    pub fn new(
        label: impl Into<String>,
        texture_id: Option<TextureId>,
        format: Option<TextureViewFormat>,
        dimension: Option<wgpu::TextureViewDimension>,
    ) -> TextureView {
        TextureView {
            label: label.into(),
            format,
            dimension,
            texture_id,
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    resource_getters! {
        pub fn texture_id() -> Option<TextureId>;
        pub fn format() -> Option<TextureViewFormat>;
        pub fn dimension() -> Option<wgpu::TextureViewDimension>;
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        pub fn set_label(label: String);
        pub fn set_texture_id(texture_id: Option<TextureId>);
        pub fn set_format(format: Option<TextureViewFormat>);
        pub fn set_dimension(dimension: Option<wgpu::TextureViewDimension>);
    }

    fn create_view(
        label: &str,
        texture: &Texture,
        runtime: &TextureRuntime,
        format: Option<TextureViewFormat>,
        dimension: Option<wgpu::TextureViewDimension>,
        downlevel_flags: wgpu::DownlevelFlags,
    ) -> wgpu::TextureView {
        let supports_view_formats = downlevel_flags.contains(wgpu::DownlevelFlags::VIEW_FORMATS);

        let wgpu_format = match (supports_view_formats, format) {
            (true, Some(TextureViewFormat::Srgb)) => Some(texture.format().add_srgb_suffix()),
            (true, Some(TextureViewFormat::Linear)) => Some(texture.format().remove_srgb_suffix()),
            _ => None,
        };

        let view = runtime.inner().create_view(&wgpu::TextureViewDescriptor {
            label: Some(&label),
            format: wgpu_format,
            dimension,
            ..Default::default()
        });

        view
    }
}

impl TextureViewRuntime {
    pub fn inner(&self) -> &wgpu::TextureView {
        &self.inner
    }

    /// Returns the egui texture ID.
    /// Only returns `Some` if the texture format is `Rgba8UnormSrgb` due to egui texture format requirements.
    pub fn egui_id(&self) -> Option<egui::TextureId> {
        self.egui_id
    }
}

impl Creatable for TextureView {
    fn create(label: String) -> Self {
        Self::new(label, None, None, None)
    }
}

impl ProjectResource for TextureView {
    type Id = TextureViewId;

    fn label(&self) -> &str {
        &self.label
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

impl SyncResource for TextureView {
    type Context<'a> = TextureViewCreationContext<'a>;
    type Runtime = TextureViewRuntime;
    type Job = TextureViewJob;

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        match job {
            TextureViewJob::Start => {
                let previous_egui_id = previous.as_ref().and_then(|runtime| runtime.egui_id);
                self.sync(
                    ctx,
                    None,
                    TextureViewJob::PendingResource { previous_egui_id },
                )
            }
            TextureViewJob::PendingResource { previous_egui_id } => {
                let texture_id = self
                    .texture_id
                    .ok_or(AppError::uninit_field("Texture Id"))?;

                let texture = ctx.textures.get(texture_id)?;
                let Some(runtime_texture) = ctx.textures_runtime.get_init(texture_id)? else {
                    return Ok(SyncOutcome::Pending(TextureViewJob::PendingResource {
                        previous_egui_id,
                    }));
                };

                let scope = WgpuErrorScope::push(ctx.device);

                let inner = Self::create_view(
                    &self.label,
                    texture,
                    runtime_texture,
                    self.format,
                    self.dimension,
                    ctx.downlevel_flags,
                );

                let has_correct_format = ALLOWED_EGUI_FORMATS.contains(&texture.format());

                let egui_id = match (previous_egui_id, has_correct_format) {
                    (Some(egui_id), true) => {
                        ctx.egui_renderer.update_egui_texture(
                            ctx.device,
                            &inner,
                            wgpu::FilterMode::Linear,
                            egui_id,
                        );
                        Some(egui_id)
                    }
                    (Some(egui_id), false) => {
                        ctx.egui_renderer.remove_egui_texture(egui_id);
                        None
                    }
                    (None, true) => Some(ctx.egui_renderer.register_egui_texture(
                        ctx.device,
                        &inner,
                        wgpu::FilterMode::Linear,
                    )),
                    (None, false) => None,
                };

                let runtime = TextureViewRuntime { inner, egui_id };
                self.sync(ctx, None, TextureViewJob::Validation(runtime, scope.pop()))
            }
            TextureViewJob::Validation(runtime, mut future) => match future.try_resolve() {
                Poll::Ready(result) => result.map(|()| SyncOutcome::Changed(runtime)),
                Poll::Pending => Ok(SyncOutcome::Pending(TextureViewJob::Validation(
                    runtime, future,
                ))),
            },
        }
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        let Some(texture_id) = self.texture_id else {
            return false;
        };
        tracker.was_changed(texture_id)
    }
}
