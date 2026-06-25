// An avenue of instanced cubes, drawn in a single instanced draw call with no
// vertex buffers. `vertex_index` walks the 36 vertices of a unit cube built
// procedurally; `instance_index` places that cube on the grid in two staggered
// rows receding down the corridor.
//
// CUBES_PER_ROW and ROWS must match the constants in src/scene/depth_testing.rs,
// which derive the instance count of the draw.

struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

const CUBES_PER_ROW: u32 = 6u;
const ROWS: u32 = 2u;
const SPACING: f32 = 3.0;     // gap between cubes along the avenue (-z)
const ROW_OFFSET: f32 = 1.5;  // half-width of the corridor (rows at +/- this x)
const FIRST_Z: f32 = -2.0;    // z of the nearest cube
const CUBE_HALF: f32 = 0.5;

// Unit cube, half-extent 0.5: 6 faces, 2 triangles each. Winding is irrelevant
// since the pipeline disables culling.
fn cube_position(vertex_index: u32) -> vec3<f32> {
    var p = array<vec3<f32>, 36>(
        // +X
        vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5),
        vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5), vec3<f32>( 0.5, -0.5,  0.5),
        // -X
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(-0.5,  0.5, -0.5), vec3<f32>(-0.5,  0.5,  0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(-0.5,  0.5,  0.5), vec3<f32>(-0.5, -0.5,  0.5),
        // +Y
        vec3<f32>(-0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5),
        vec3<f32>(-0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5), vec3<f32>(-0.5,  0.5,  0.5),
        // -Y
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5,  0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5,  0.5), vec3<f32>(-0.5, -0.5,  0.5),
        // +Z
        vec3<f32>(-0.5, -0.5,  0.5), vec3<f32>( 0.5, -0.5,  0.5), vec3<f32>( 0.5,  0.5,  0.5),
        vec3<f32>(-0.5, -0.5,  0.5), vec3<f32>( 0.5,  0.5,  0.5), vec3<f32>(-0.5,  0.5,  0.5),
        // -Z
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5), vec3<f32>(-0.5,  0.5, -0.5),
    );
    return p[vertex_index];
}

fn cube_normal(face: u32) -> vec3<f32> {
    var n = array<vec3<f32>, 6>(
        vec3<f32>( 1.0,  0.0,  0.0),
        vec3<f32>(-1.0,  0.0,  0.0),
        vec3<f32>( 0.0,  1.0,  0.0),
        vec3<f32>( 0.0, -1.0,  0.0),
        vec3<f32>( 0.0,  0.0,  1.0),
        vec3<f32>( 0.0,  0.0, -1.0),
    );
    return n[face];
}

fn palette(i: u32) -> vec3<f32> {
    var c = array<vec3<f32>, 5>(
        vec3<f32>(0.85, 0.32, 0.28),
        vec3<f32>(0.30, 0.55, 0.85),
        vec3<f32>(0.95, 0.72, 0.25),
        vec3<f32>(0.42, 0.75, 0.45),
        vec3<f32>(0.70, 0.45, 0.80),
    );
    return c[i % 5u];
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let local = cube_position(vertex_index);
    let normal = cube_normal(vertex_index / 6u);

    // Grid placement: alternate rows, marching down -z. The second row is
    // staggered by half a spacing so the cubes interleave.
    let row = instance_index % ROWS;
    let col = instance_index / ROWS;
    let x = select(-ROW_OFFSET, ROW_OFFSET, row == 1u);
    let z_stagger = select(0.0, SPACING * 0.5, row == 1u);
    let z = FIRST_Z - f32(col) * SPACING - z_stagger;
    // y so the cube sits on the floor (its bottom face at y=0).
    let offset = vec3<f32>(x, CUBE_HALF, z);

    let world = local + offset;

    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(world, 1.0);
    out.world_normal = normal;
    out.color = palette(instance_index);
    return out;
}

const SUN_DIR: vec3<f32> = vec3<f32>(0.4, 0.9, 0.35);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let diffuse = max(dot(n, normalize(SUN_DIR)), 0.0);
    let shade = 0.25 + 0.75 * diffuse; // ambient + lambert
    return vec4<f32>(in.color * shade, 1.0);
}
