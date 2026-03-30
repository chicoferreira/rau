// Vertex shader

struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
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
    @location(2) world_view_position: vec3<f32>,
    @location(3) world_light_position: vec3<f32>,
    @location(4) world_normal: vec3<f32>,
    @location(5) world_tangent: vec3<f32>,
    @location(6) world_bitangent: vec3<f32>,
}

const SPACE_BETWEEN_INSTANCES: f32 = 3.0;
const NUM_INSTANCES_PER_ROW: u32 = 10;

@vertex
fn vs_main(
    model: VertexInput,
    @builtin(instance_index) instance_id: u32
) -> VertexOutput {
    let x = f32(instance_id % NUM_INSTANCES_PER_ROW) * SPACE_BETWEEN_INSTANCES;
    let z = f32(instance_id / NUM_INSTANCES_PER_ROW) * SPACE_BETWEEN_INSTANCES;
    let instance_position = vec3<f32>(x, 0.0, z);
    let instance_translation = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(instance_position, 1.0),
    );

    let rotation_matrix = mat3x3<f32>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
    ); // TODO

    let world_position = instance_translation * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.tex_coords = model.tex_coords;
    out.world_normal = normalize(rotation_matrix * model.normal);
    out.world_tangent = normalize(rotation_matrix * model.tangent);
    out.world_bitangent = normalize(rotation_matrix * model.bitangent);
    out.world_position = world_position.xyz;
    out.world_view_position = camera.view_pos.xyz;
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var t_normal: texture_2d<f32>;
@group(0) @binding(3)
var s_normal: sampler;

@group(3)
@binding(0)
var env_map: texture_cube<f32>;
@group(3)
@binding(1)
var env_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let object_normal: vec4<f32> = textureSample(t_normal, s_normal, in.tex_coords);

    let world_tangent = normalize(in.world_tangent - dot(in.world_tangent, in.world_normal) * in.world_normal);
    let world_bitangent = cross(world_tangent, in.world_normal);

    let TBN = mat3x3(
        world_tangent,
        world_bitangent,
        in.world_normal,
    );
    let tangent_normal = object_normal.xyz * 2.0 - 1.0;
    let world_normal = TBN * tangent_normal;

    let light_dir = normalize(light.position - in.world_position);
    let view_dir = normalize(in.world_view_position - in.world_position);
    let half_dir = normalize(view_dir + light_dir);

    let diffuse_strength = max(dot(world_normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let specular_strength = pow(max(dot(world_normal, half_dir), 0.0), 32.0);
    let specular_color = specular_strength * light.color;

    let world_reflect = reflect(-view_dir, world_normal);
    let reflection = textureSample(env_map, env_sampler, world_reflect).rgb;
    let shininess = 0.1;

    let result = (diffuse_color + specular_color) * object_color.xyz + reflection * shininess;

    return vec4<f32>(result, object_color.a);
}
