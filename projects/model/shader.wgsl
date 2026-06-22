struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}

@group(1) @binding(0)
var<uniform> camera: Camera;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}

@group(2) @binding(0)
var<uniform> light: Light;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

const MODEL_SCALE: f32 = 0.02;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    let world_position = model.position * MODEL_SCALE;

    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(world_position, 1.0);
    out.tex_coords = model.tex_coords;
    out.world_position = world_position;
    out.normal = model.normal;
    out.tangent = model.tangent;
    out.bitangent = model.bitangent;
    return out;
}

@group(0) @binding(0)
var diffuse_texture: texture_2d<f32>;

@group(0) @binding(1)
var normal_texture: texture_2d<f32>;

@group(0) @binding(2)
var specular_texture: texture_2d<f32>;

@group(0) @binding(3)
var material_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(diffuse_texture, material_sampler, in.tex_coords).rgb;
    let tangent_normal = textureSample(normal_texture, material_sampler, in.tex_coords).xyz * 2.0 - 1.0;
    let specular_color = textureSample(specular_texture, material_sampler, in.tex_coords).rgb;

    let normal = normalize(in.normal);
    let tangent = normalize(in.tangent - dot(in.tangent, normal) * normal);
    let handedness = select(-1.0, 1.0, dot(cross(normal, tangent), in.bitangent) >= 0.0);
    let bitangent = normalize(cross(normal, tangent)) * handedness;
    let world_normal = normalize(mat3x3<f32>(tangent, bitangent, normal) * tangent_normal);

    let to_light = light.position - in.world_position;
    let light_distance_squared = max(dot(to_light, to_light), 0.01);
    let light_direction = normalize(to_light);
    let view_direction = normalize(camera.position.xyz - in.world_position);
    let half_direction = normalize(light_direction + view_direction);

    let radiance = light.color * (8.0 / light_distance_squared);
    let diffuse = max(dot(world_normal, light_direction), 0.0) * radiance;
    let specular = pow(max(dot(world_normal, half_direction), 0.0), 64.0)
        * radiance
        * specular_color
        * 0.8;
    let ambient = vec3<f32>(0.055, 0.065, 0.085);

    return vec4<f32>(albedo * (ambient + diffuse) + specular, 1.0);
}
