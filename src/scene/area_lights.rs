//! Three glowing panels in a dark room, lit with Linearly Transformed Cosines.
//!
//! Each panel is a real polygonal light: the shading integrates the whole
//! rectangle instead of approximating it with point lights, so the floor picks
//! up soft shadow edges and long glossy streaks that follow the panels around.
//!
//! Ported from LearnOpenGL's "Area Lights" guest article
//! (<https://learnopengl.com/Guest-Articles/2022/Area-Lights>), which implements:
//!
//!   Real-Time Polygonal-Light Shading with Linearly Transformed Cosines.
//!   Eric Heitz, Jonathan Dupuy, Stephen Hill and David Neubelt.
//!   ACM Transactions on Graphics (Proceedings of ACM SIGGRAPH 2016) 35(4), 2016.
//!
//! The room is ours — the paper's own figures use Crytek Sponza and an
//! unpublished Unity scene, so this one is five procedural quads instead. The
//! only assets are the two fitted lookup tables, stored as 32-bit float EXR
//! because their values go negative and past 1.0.

use crate::{
    error::AppResult,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::{Camera, Deg, Fov},
            dimension::Dimension,
            render_pass::{Color, LoadOperation, RenderPass, RenderPassTarget},
            render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
            sampler::{Sampler, SamplerSpec},
            shader::Shader,
            texture::{Texture, TextureSource},
            texture_view::{TextureView, TextureViewFormat},
            uniform::{
                Transform, Uniform, UniformField, UniformFieldData, UniformFieldSource,
                camera::CameraField,
            },
            viewport::Viewport,
        },
    },
    utils::wgpu_utils::{AddressMode, PrimitiveState, TextureFormat},
};

struct AreaLightSpec {
    /// Places the panel: scale is its width and height, position its centre.
    transform: Transform,
    color: [f32; 3],
    intensity: f32,
}

const AREA_LIGHTS: [AreaLightSpec; 3] = [
    AreaLightSpec {
        transform: Transform {
            position: [-2.5, 1.75, -2.98],
            rotation: [0.0; 3],
            scale: [0.9, 2.0, 1.0],
        },
        color: [1.0, 0.45, 0.12],
        intensity: 4.0,
    },
    AreaLightSpec {
        transform: Transform {
            position: [0.0, 1.15, -2.98],
            rotation: [0.0; 3],
            scale: [2.2, 0.85, 1.0],
        },
        color: [0.45, 1.0, 0.5],
        intensity: 4.0,
    },
    AreaLightSpec {
        transform: Transform {
            position: [2.5, 1.75, -2.98],
            rotation: [0.0; 3],
            scale: [0.9, 2.0, 1.0],
        },
        color: [1.0, 0.75, 0.25],
        intensity: 4.0,
    },
];

/// Floor, ceiling, back, left and right, six vertices each.
const ROOM_VERTICES: u32 = 5 * 6;

pub async fn create_scene() -> AppResult<Project> {
    let mut project = Project::default();

    let room_shader_id = project
        .shaders
        .register(Shader::new("Room Shader", FilePath::from_str("room.wgsl")?));
    let panel_shader_id = project.shaders.register(Shader::new(
        "Light Panel Shader",
        FilePath::from_str("panel.wgsl")?,
    ));

    let dimension_id = project
        .dimensions
        .register(Dimension::new_runtime("Main Dimension"));

    // Standing in the room, panels across the top of the frame and the floor
    // they light across the bottom.
    let mut camera = Camera::new("Camera".to_string());
    camera.set_dimension_id(Some(dimension_id));
    camera.set_fovy(Fov::new(Deg(60.0)));
    camera.look_at(
        glam::Vec3::new(0.0, 1.45, 2.0),
        glam::Vec3::new(0.0, 1.0, -2.2),
    );
    let camera_id = project.cameras.register(camera);

    let camera_uniform_id = project.uniforms.register(Uniform::new(
        "Camera",
        vec![
            UniformField::new(
                "position",
                UniformFieldSource::new_camera_sourced(Some(camera_id), CameraField::Position),
            ),
            UniformField::new(
                "projection_view",
                UniformFieldSource::new_camera_sourced(
                    Some(camera_id),
                    CameraField::ProjectionView,
                ),
            ),
        ],
    ));
    let camera_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Camera Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(camera_uniform_id)),
        )],
    ));

    // All three lights share one uniform — the shaders read them as an array,
    // and a pipeline only gets four bind groups. Field order and types have to
    // match `AreaLight` in the shaders.
    let mut light_fields = Vec::new();
    for (index, light) in AREA_LIGHTS.iter().enumerate() {
        light_fields.push(UniformField::new(
            format!("light_{index}_transform"),
            UniformFieldSource::new_transform(light.transform),
        ));
        light_fields.push(UniformField::new(
            format!("light_{index}_color"),
            UniformFieldSource::new_user_defined(UniformFieldData::Rgb(light.color)),
        ));
        light_fields.push(UniformField::new(
            format!("light_{index}_intensity"),
            UniformFieldSource::new_user_defined(UniformFieldData::Float(light.intensity)),
        ));
    }

    let lights_uniform_id = project
        .uniforms
        .register(Uniform::new("Area Lights", light_fields));
    let lights_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Area Lights Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(lights_uniform_id)),
        )],
    ));

    // A glossy floor and rougher walls. `specular` is the reflectance the
    // LearnOpenGL article uses.
    let material_uniform_id = project.uniforms.register(Uniform::new(
        "Material",
        vec![
            UniformField::new(
                "floor_albedo",
                UniformFieldSource::new_user_defined(UniformFieldData::Rgb([0.22, 0.23, 0.25])),
            ),
            UniformField::new(
                "wall_albedo",
                UniformFieldSource::new_user_defined(UniformFieldData::Rgb([0.35, 0.34, 0.32])),
            ),
            UniformField::new(
                "floor_roughness",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.12)),
            ),
            UniformField::new(
                "wall_roughness",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.6)),
            ),
            UniformField::new(
                "specular",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.23)),
            ),
            UniformField::new(
                "exposure",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(1.0)),
            ),
            UniformField::new(
                "ambient",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.02)),
            ),
        ],
    ));
    let material_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Material Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(material_uniform_id)),
        )],
    ));

    // The fitted tables: inverse LTC matrices in one, GGX norm, Fresnel and the
    // horizon-clipped sphere in the other. Clamped and bilinear, as the fit
    // assumes.
    let ltc_sampler_id = project.samplers.register(Sampler::new(
        "LTC Sampler",
        SamplerSpec {
            address_mode: AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..SamplerSpec::default()
        },
    ));

    let ltc1_texture_id = project.textures.register(lut_texture(
        "LTC Matrix Table",
        FilePath::from_str("ltc1.exr")?,
    ));
    let ltc2_texture_id = project.textures.register(lut_texture(
        "LTC Norm Table",
        FilePath::from_str("ltc2.exr")?,
    ));
    let ltc1_view_id = project.texture_views.register(TextureView::new(
        "LTC Matrix Table View",
        Some(ltc1_texture_id),
        None,
        None,
    ));
    let ltc2_view_id = project.texture_views.register(TextureView::new(
        "LTC Norm Table View",
        Some(ltc2_texture_id),
        None,
        None,
    ));

    let lut = |view_id| BindGroupResource::Texture {
        texture_view_id: Some(view_id),
        view_dimension: wgpu::TextureViewDimension::D2,
        sample_type: wgpu::TextureSampleType::Float { filterable: true },
    };
    let ltc_bind_group_id = project.bind_groups.register(BindGroup::new(
        "LTC Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(lut(ltc1_view_id)),
            BindGroupEntry::new_vertex_fragment(lut(ltc2_view_id)),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(ltc_sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
        ],
    ));

    let color_format = TextureFormat::Rgba8UnormSrgb;
    let viewport_texture_id = project.textures.register(Texture::new(
        "Viewport Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::dimension(dimension_id),
    ));
    let render_texture_view_id = project.texture_views.register(TextureView::new(
        "Viewport",
        Some(viewport_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let viewport_id = project.viewports.register(Viewport::new(
        "Area Lights Viewport",
        Some(render_texture_view_id),
        Some(dimension_id),
        Some(camera_id),
    ));

    let depth_format = TextureFormat::Depth32Float;
    let depth_buffer_texture_id = project.textures.register(Texture::new(
        "Depth Texture",
        depth_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::dimension(dimension_id),
    ));
    let depth_buffer_view_id = project.texture_views.register(TextureView::new(
        "Depth Texture View",
        Some(depth_buffer_texture_id),
        None,
        None,
    ));

    // The camera is inside the box, so the room is seen from the back of its
    // faces — culling is off rather than flipping the winding.
    let primitive = PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        polygon_mode: wgpu::PolygonMode::Fill,
    };

    let room_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Room Pipeline",
        primitive,
        Some(room_shader_id),
        Some(room_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..ROOM_VERTICES,
            instances: 0..1,
        },
        vec![
            BindGroupTarget::Static(ltc_bind_group_id),
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(lights_bind_group_id),
            BindGroupTarget::Static(material_bind_group_id),
        ],
        color_format,
        Some(depth_format),
    ));

    let panel_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Light Panel Pipeline",
        primitive,
        Some(panel_shader_id),
        Some(panel_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..(AREA_LIGHTS.len() as u32 * 6),
            instances: 0..1,
        },
        vec![
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(lights_bind_group_id),
            BindGroupTarget::Static(material_bind_group_id),
        ],
        color_format,
        Some(depth_format),
    ));

    let mut render_pass = RenderPass::new(
        "Area Lights Render Pass",
        RenderPassTarget::new(
            Some(render_texture_view_id),
            LoadOperation::Clear(Color([0.0, 0.0, 0.0, 1.0])),
        ),
        Some(RenderPassTarget::new(
            Some(depth_buffer_view_id),
            LoadOperation::Clear(1.0),
        )),
    );
    render_pass.set_pipelines(vec![room_pipeline_id, panel_pipeline_id]);
    let render_pass_id = project.render_passes.register(render_pass);

    project.presentation.set_render_passes(vec![render_pass_id]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(project)
}

fn lut_texture(label: &str, path: FilePath) -> Texture {
    Texture::new(
        label,
        TextureFormat::Rgba32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        TextureSource::Image(Some(path)),
    )
}
