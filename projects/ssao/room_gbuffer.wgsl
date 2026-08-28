// Room G-buffer. A unit cube scaled to a 15-unit box and lifted to y = 7 (floor
// at y = -0.5), matching `ssao.cpp`'s room. It is seen from the inside, so the
// outward face normals are flipped inward, like `ssao.cpp`'s `invertedNormals`
// flag. Writes view-space position and normal to the pass's two targets.
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

const ROOM_SCALE: f32 = 15.0;
const ROOM_CENTER: vec3<f32> = vec3<f32>(0.0, 7.0, 0.0);

// Unit cube, half-extent 0.5: 6 faces, 2 triangles each.
fn cube_position(vertex_index: u32) -> vec3<f32> {
    var p = array<vec3<f32>, 36>(
        vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5),
        vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5), vec3<f32>( 0.5, -0.5,  0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(-0.5,  0.5, -0.5), vec3<f32>(-0.5,  0.5,  0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(-0.5,  0.5,  0.5), vec3<f32>(-0.5, -0.5,  0.5),
        vec3<f32>(-0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5),
        vec3<f32>(-0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5), vec3<f32>(-0.5,  0.5,  0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5,  0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5,  0.5), vec3<f32>(-0.5, -0.5,  0.5),
        vec3<f32>(-0.5, -0.5,  0.5), vec3<f32>( 0.5, -0.5,  0.5), vec3<f32>( 0.5,  0.5,  0.5),
        vec3<f32>(-0.5, -0.5,  0.5), vec3<f32>( 0.5,  0.5,  0.5), vec3<f32>(-0.5,  0.5,  0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5), vec3<f32>(-0.5,  0.5, -0.5),
    );
    return p[vertex_index];
}

// Outward face normals; negated in the vertex shader so they face inward.
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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) view_position: vec3<f32>,
    @location(1) view_normal: vec3<f32>,
}

// The two color targets of the G-Buffer pass, in order.
struct GBuffer {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let world = cube_position(vertex_index) * ROOM_SCALE + ROOM_CENTER;
    // Flip the normal inward, since we shade the inside of the room.
    let world_normal = -cube_normal(vertex_index / 6u);

    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(world, 1.0);
    out.view_position = (camera.view * vec4<f32>(world, 1.0)).xyz;
    out.view_normal = (camera.view * vec4<f32>(world_normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> GBuffer {
    var out: GBuffer;
    out.position = vec4<f32>(in.view_position, 1.0);
    out.normal = vec4<f32>(normalize(in.view_normal), 1.0);
    return out;
}
