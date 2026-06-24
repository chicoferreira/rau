// Displays the current grid on a full-screen triangle. Each fragment looks up the
// cell underneath it with `textureLoad` (nearest sampling, so cells stay crisp)
// and maps live/dead to two colours.

@group(0) @binding(0)
var grid: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
    // Oversized triangle that covers the screen.
    let uv = vec2<f32>(f32((id << 1u) & 2u), f32(id & 2u));
    var out: VertexOutput;
    out.position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(grid));
    let coord = clamp(vec2<i32>(in.uv * vec2<f32>(dims)), vec2<i32>(0), dims - vec2<i32>(1));
    let alive = textureLoad(grid, coord, 0).r;

    let dead_color = vec3<f32>(0 / 255.0, 10 / 255.0, 40 / 255.0);
    let alive_color = vec3<f32>(255 / 255.0, 190 / 255.0, 0 / 255.0);
    let color = mix(dead_color, alive_color, alive);

    return vec4<f32>(color, 1.0);
}
