struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) clip_position: vec4<f32>,
};

@group(0) @binding(0)
var input_texture: texture_2d<f32>;
@group(0) @binding(1)
var tex_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    out.clip_position = vec4<f32>(out.uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

@fragment
fn fs_main(vs: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(input_texture));
    let texel = 1.0 / tex_size.x;

    var color = textureSample(input_texture, tex_sampler, vs.uv) * 0.227027;
    color += textureSample(input_texture, tex_sampler, vs.uv + vec2<f32>(texel * 1.0, 0.0)) * 0.1945946;
    color += textureSample(input_texture, tex_sampler, vs.uv - vec2<f32>(texel * 1.0, 0.0)) * 0.1945946;
    color += textureSample(input_texture, tex_sampler, vs.uv + vec2<f32>(texel * 2.0, 0.0)) * 0.1216216;
    color += textureSample(input_texture, tex_sampler, vs.uv - vec2<f32>(texel * 2.0, 0.0)) * 0.1216216;
    color += textureSample(input_texture, tex_sampler, vs.uv + vec2<f32>(texel * 3.0, 0.0)) * 0.054054;
    color += textureSample(input_texture, tex_sampler, vs.uv - vec2<f32>(texel * 3.0, 0.0)) * 0.054054;
    color += textureSample(input_texture, tex_sampler, vs.uv + vec2<f32>(texel * 4.0, 0.0)) * 0.016216;
    color += textureSample(input_texture, tex_sampler, vs.uv - vec2<f32>(texel * 4.0, 0.0)) * 0.016216;

    return color;
}
