// SSAO blur pass: a simple 4x4 box blur over the raw occlusion. The per-pixel
// random rotation in the SSAO pass tiles over 4x4, so averaging a 4x4 window
// removes that noise almost perfectly while keeping the occlusion soft.
//
// Ported from https://learnopengl.com/Advanced-Lighting/SSAO (CC BY-NC 4.0).

@group(0) @binding(0) var ssao_input: texture_2d<f32>;
@group(0) @binding(1) var ssao_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    out.clip_position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(ssao_input));
    var result = 0.0;
    for (var x = -2; x < 2; x = x + 1) {
        for (var y = -2; y < 2; y = y + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            result = result + textureSample(ssao_input, ssao_sampler, in.uv + offset).r;
        }
    }
    result = result / 16.0;
    return vec4<f32>(result, result, result, 1.0);
}
