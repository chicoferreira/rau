struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) clip_position: vec4<f32>,
};

@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;
@group(0) @binding(1)
var tex_sampler: sampler;

struct BloomSettings {
    threshold: f32,
    intensity: f32,
};
@group(0) @binding(2)
var<uniform> bloom: BloomSettings;

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
    let color = textureSample(hdr_texture, tex_sampler, vs.uv);
    let brightness = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let contribution = max(brightness - bloom.threshold, 0.0);
    let factor = contribution / max(brightness, 0.001);
    return vec4<f32>(color.rgb * factor, 1.0);
}
