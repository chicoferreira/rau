// The checkered floor under the avenue: a single quad on the y=0 plane built
// from the vertex index, no vertex buffer. The checker pattern gives the eye
// (and the depth view) a clear sense of receding distance. Uses only the camera
// (group 0).

struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

// Half-extent of the floor quad. Comfortably larger than the avenue so the
// cubes never overhang bare ground.
const HALF: f32 = 34.0;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_xz: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-HALF, -HALF),
        vec2<f32>(HALF, -HALF),
        vec2<f32>(HALF, HALF),
        vec2<f32>(-HALF, -HALF),
        vec2<f32>(HALF, HALF),
        vec2<f32>(-HALF, HALF),
    );
    let p = corners[vertex_index];

    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(p.x, 0.0, p.y, 1.0);
    out.world_xz = p;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let cell = vec2<i32>(floor(in.world_xz));
    let checker = (cell.x + cell.y) & 1;
    let base = select(vec3<f32>(0.28, 0.29, 0.32), vec3<f32>(0.38, 0.40, 0.43), checker == 1);
    return vec4<f32>(base, 1.0);
}
