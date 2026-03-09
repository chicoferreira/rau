use crate::{
    project::{
        self, BindGroupId, SamplerId, TextureViewId,
        bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
    },
    state,
};

pub struct HdrPipeline {
    pipeline: wgpu::RenderPipeline,
    pub bind_group_id: BindGroupId,
}

impl HdrPipeline {
    pub const RENDER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

    pub fn new(
        device: &wgpu::Device,
        project: &mut project::Project,
        output_texture_view_id: TextureViewId,
        output_format: wgpu::TextureFormat,
        sampler_id: SamplerId,
        hdr_shader_id: project::ShaderId,
    ) -> anyhow::Result<Self> {
        let bind_group = BindGroup::new(
            project,
            device,
            "HDR Bind Group".to_string(),
            vec![
                BindGroupEntry {
                    resource: BindGroupResource::Texture {
                        texture_view_id: output_texture_view_id,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                },
                BindGroupEntry {
                    resource: BindGroupResource::Sampler {
                        sampler_id,
                        sampler_binding_type: wgpu::SamplerBindingType::Filtering,
                    },
                },
            ],
        );

        let shader = project.shaders.get(hdr_shader_id).unwrap();
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_group.inner_layout()],
            immediate_size: 0,
        });

        let bind_group_id = project.bind_groups.register(bind_group);

        let pipeline = state::create_render_pipeline(
            "hdr pipeline",
            device,
            &pipeline_layout,
            output_format,
            None,
            &[],
            wgpu::PrimitiveTopology::TriangleList,
            shader.create_wgpu_shader_module(device)?,
        );

        Ok(Self {
            pipeline,
            bind_group_id,
        })
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }
}
