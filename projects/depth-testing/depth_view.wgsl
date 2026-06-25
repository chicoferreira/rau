// Depth-buffer visualisation. A full-screen triangle samples the depth buffer
// written by the scene pass, linearises the non-linear [0, 1] depth back to
// eye-space distance, and writes it as grayscale: near = dark, far = bright.
//
// The depth buffer is sampled with textureLoad (no sampler needed), indexed by
// the fragment's pixel coordinates, since the depth texture is the same size as
// this render target.

@group(0) @binding(0)
var depth_texture: texture_depth_2d;

struct DepthParams {
    near: f32,
    far: f32,
}
@group(0) @binding(1)
var<uniform> params: DepthParams;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Oversized triangle that covers the whole screen.
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(corners[vertex_index], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    // frag_coord.xy is the pixel centre in framebuffer space; the depth texture
    // shares this resolution, so we can index it directly.
    let d = textureLoad(depth_texture, vec2<i32>(frag_coord.xy), 0);

    // Reconstruct eye-space distance from a [0, 1] (wgpu/D3D-style) depth value.
    // At d=0 this is `near`, at d=1 it is `far`.
    let near = params.near;
    let far = params.far;
    let linear = (near * far) / (far - d * (far - near));

    // Map [near, far] -> [0, 1] so near is dark and far is bright.
    let g = clamp((linear - near) / (far - near), 0.0, 1.0);
    return vec4<f32>(g, g, g, 1.0);
}
