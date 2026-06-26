const PI: f32 = 3.14159265;
const SEGMENTS: u32 = 20u;
const RINGS: u32 = 10u;
const GRID: u32 = 20u;
const SPACING: f32 = 1.1;
const RADIUS: f32 = 0.5;

struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

fn hash_f(x: f32) -> f32 { return fract(sin(x * 127.1) * 43758.5453); }

fn sphere_point(s: u32, r: u32) -> vec3<f32> {
    let theta = f32(s) / f32(SEGMENTS) * 2.0 * PI;
    let phi = f32(r) / f32(RINGS) * PI;
    return vec3<f32>(sin(phi) * cos(theta), cos(phi), sin(phi) * sin(theta));
}

fn sphere_vertex(vi: u32) -> vec3<f32> {
    let quad = vi / 6u;
    let corner = vi % 6u;
    let seg = quad % SEGMENTS;
    let ring = quad / SEGMENTS;
    var ds: u32; var dr: u32;
    switch corner {
        case 0u { ds = 0u; dr = 0u; }
        case 1u { ds = 0u; dr = 1u; }
        case 2u { ds = 1u; dr = 0u; }
        case 3u { ds = 1u; dr = 0u; }
        case 4u { ds = 0u; dr = 1u; }
        case 5u { ds = 1u; dr = 1u; }
        default { ds = 0u; dr = 0u; }
    }
    return sphere_point(seg + ds, ring + dr);
}

fn sphere_center(ii: u32) -> vec3<f32> {
    let gx = ii % GRID;
    let gz = ii / GRID;
    let jx = (hash_f(f32(ii) * 3.17) - 0.5) * 0.15;
    let jz = (hash_f(f32(ii) * 7.31) - 0.5) * 0.15;
    return vec3<f32>(
        (f32(gx) - f32(GRID - 1u) * 0.5) * SPACING + jx,
        RADIUS,
        (f32(gz) - f32(GRID - 1u) * 0.5) * SPACING + jz,
    );
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VertexOutput {
    let local = sphere_vertex(vi) * RADIUS;
    let world = local + sphere_center(ii);
    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(world, 1.0);
    out.world_position = world;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.world_position, 1.0);
}
