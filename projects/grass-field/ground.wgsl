// The ground plane under the grass: a single quad on the y=0 plane, built from
// the vertex index with no vertex buffer. Uses only the camera (group 0).

struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

// Half-extent of the ground quad. Covers the full grass field
// (BLADES_PER_ROW * BLADE_SPACING = 60 units across) so the blades never
// overhang bare ground.
const HALF: f32 = 30.0;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_xz: vec2<f32>,
};

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
    // Sit a hair below y=0 so the blade bases never z-fight with the ground.
    out.clip_position = camera.projection_view * vec4<f32>(p.x, -0.01, p.y, 1.0);
    out.world_xz = p;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = clamp(length(in.world_xz) / HALF, 0.0, 1.0);
    let near = vec3<f32>(0.06, 0.10, 0.03);
    let far = vec3<f32>(0.03, 0.05, 0.02);
    return vec4<f32>(mix(near, far, d), 1.0);
}
