// Backpack G-buffer (normals). Same model draw as `backpack_position.wgsl`, but
// writes view-space normals. The backpack transform is a rotation + translation
// (uniform scale 1), so it is rigid and the normal can be transformed by the
// model matrix directly — no inverse-transpose needed.
//
// Ported from https://learnopengl.com/Advanced-Lighting/SSAO (CC BY-NC 4.0).

struct Camera {
    position: vec4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    projection_view: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct Transform {
    model: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> transform: Transform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) view_normal: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let world = (transform.model * vec4<f32>(in.position, 1.0)).xyz;
    let world_normal = (transform.model * vec4<f32>(in.normal, 0.0)).xyz;

    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(world, 1.0);
    out.view_normal = (camera.view * vec4<f32>(world_normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(normalize(in.view_normal), 1.0);
}
