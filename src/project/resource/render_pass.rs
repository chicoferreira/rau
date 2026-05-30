use serde::{Deserialize, Serialize};

use crate::project::{
    Creatable, ProjectResource, RenderPassId, RenderPipelineId, TextureViewId, sync::Revision,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPass {
    label: String,
    target: RenderPassTarget<Color>,
    depth_target: Option<RenderPassTarget<f32>>,
    pipelines: Vec<RenderPipelineId>,
    #[serde(skip)]
    project_revision: Revision,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPassTarget<T> {
    texture_view_id: Option<TextureViewId>,
    load_operation: LoadOperation<T>,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum LoadOperation<T> {
    Clear(T),
    Load,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Color(pub [f32; 4]);

impl Creatable for RenderPass {
    fn create(label: String) -> Self {
        Self {
            label,
            target: Default::default(),
            depth_target: Default::default(),
            pipelines: Default::default(),
            project_revision: Default::default(),
        }
    }
}

impl ProjectResource for RenderPass {
    type Id = RenderPassId;

    fn label(&self) -> &str {
        &self.label
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

impl RenderPass {
    pub fn new(
        label: impl Into<String>,
        target: RenderPassTarget<Color>,
        depth_target: Option<RenderPassTarget<f32>>,
    ) -> Self {
        Self {
            label: label.into(),
            target,
            depth_target,
            pipelines: Default::default(),
            project_revision: Default::default(),
        }
    }

    pub fn target(&self) -> RenderPassTarget<Color> {
        self.target
    }

    pub fn depth_target(&self) -> Option<RenderPassTarget<f32>> {
        self.depth_target
    }

    pub fn set_label(&mut self, label: String) {
        if self.label != label {
            self.label = label;
            self.project_revision.increase();
        }
    }

    pub fn set_target(&mut self, target: RenderPassTarget<Color>) {
        self.target = target;
        self.project_revision.increase();
    }

    pub fn set_depth_target(&mut self, target: Option<RenderPassTarget<f32>>) {
        self.depth_target = target;
        self.project_revision.increase();
    }

    pub fn pipelines(&self) -> &[RenderPipelineId] {
        &self.pipelines
    }

    pub fn set_pipelines(&mut self, pipelines: Vec<RenderPipelineId>) {
        if self.pipelines != pipelines {
            self.pipelines = pipelines;
            self.project_revision.increase();
        }
    }
}

impl<T> RenderPassTarget<T> {
    pub fn new(texture_view_id: Option<TextureViewId>, load_operation: LoadOperation<T>) -> Self {
        Self {
            texture_view_id,
            load_operation,
        }
    }

    pub fn texture_view_id(&self) -> Option<TextureViewId> {
        self.texture_view_id
    }

    pub fn load_operation(&self) -> LoadOperation<T>
    where
        T: Copy,
    {
        self.load_operation
    }
}

impl Default for LoadOperation<Color> {
    fn default() -> Self {
        LoadOperation::Clear(Color([0.0, 0.0, 0.0, 1.0]))
    }
}

impl Default for LoadOperation<f32> {
    fn default() -> Self {
        LoadOperation::Clear(1.0)
    }
}

impl<T> Default for RenderPassTarget<T>
where
    LoadOperation<T>: Default,
{
    fn default() -> Self {
        RenderPassTarget {
            texture_view_id: None,
            load_operation: LoadOperation::default(),
        }
    }
}

impl<T, V> From<LoadOperation<T>> for wgpu::LoadOp<V>
where
    T: Into<V>,
{
    fn from(value: LoadOperation<T>) -> Self {
        match value {
            LoadOperation::Clear(value) => wgpu::LoadOp::Clear(value.into()),
            LoadOperation::Load => wgpu::LoadOp::Load,
        }
    }
}

impl From<Color> for wgpu::Color {
    fn from(Color(value): Color) -> Self {
        wgpu::Color {
            r: value[0] as f64,
            g: value[1] as f64,
            b: value[2] as f64,
            a: value[3] as f64,
        }
    }
}
