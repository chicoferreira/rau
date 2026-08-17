// Puts the accumulation buffers on screen through a full-screen triangle.

@group(0) @binding(0)
var accumulation_red: texture_2d<f32>;
@group(0) @binding(1)
var accumulation_green: texture_2d<f32>;
@group(0) @binding(2)
var accumulation_blue: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
    // Oversized triangle that covers the screen.
    let uv = vec2<f32>(f32((id << 1u) & 2u), f32(id & 2u));
    var out: VertexOutput;
    out.position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(accumulation_red));
    let coord = clamp(vec2<i32>(in.position.xy), vec2<i32>(0), size - vec2<i32>(1));
    let color = vec3<f32>(
        textureLoad(accumulation_red, coord, 0).x,
        textureLoad(accumulation_green, coord, 0).x,
        textureLoad(accumulation_blue, coord, 0).x,
    );

    return vec4<f32>(sqrt(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0))), 1.0);
}
