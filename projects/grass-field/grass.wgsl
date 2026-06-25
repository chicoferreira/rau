// Procedurally instanced grass field.
//
// One instanced draw renders every blade. `instance_index` selects the blade —
// its grid position, height, orientation, sway phase and lean are all hashed
// from that index, so each blade is unique with no per-blade vertex data.
// `vertex_index` walks a tapered triangle strip up the blade. A `time` uniform
// drives a layered wind that scrolls across the field.
//
// BLADES_PER_ROW and SEGMENTS must match the constants in src/scene/grass.rs,
// which derive the instance and vertex counts of the draw.

struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct Grass {
    time: f32,
    wind_strength: f32,
    wind_speed: f32,
}
@group(1) @binding(0)
var<uniform> grass: Grass;

const BLADES_PER_ROW: u32 = 1000u;
const BLADE_SPACING: f32 = 0.06;
const BLADE_JITTER: f32 = 0.045;
const SEGMENTS: u32 = 5u;
const BLADE_WIDTH: f32 = 0.018;
const BLADE_HEIGHT_MIN: f32 = 0.22;
const BLADE_HEIGHT_MAX: f32 = 0.42;
const PI: f32 = 3.14159265;

// Integer hash -> [0, 1). Different `seed` values give independent attributes
// from the same blade index.
fn hash11(p: u32) -> f32 {
    var x = p;
    x = x ^ (x >> 16u);
    x = x * 0x7feb352du;
    x = x ^ (x >> 15u);
    x = x * 0x846ca68bu;
    x = x ^ (x >> 16u);
    return f32(x) / 4294967295.0;
}

fn rand(index: u32, seed: u32) -> f32 {
    return hash11(index * 747796405u + seed * 2891336453u + 1u);
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) height_t: f32,
    @location(1) world_normal: vec3<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // --- per-blade attributes, all hashed from the instance index ---
    let gx = f32(instance_index % BLADES_PER_ROW);
    let gz = f32(instance_index / BLADES_PER_ROW);
    let half = f32(BLADES_PER_ROW) * 0.5;

    let jitter_x = (rand(instance_index, 1u) - 0.5) * 2.0 * BLADE_JITTER;
    let jitter_z = (rand(instance_index, 2u) - 0.5) * 2.0 * BLADE_JITTER;
    let base = vec3<f32>(
        (gx - half) * BLADE_SPACING + jitter_x,
        0.0,
        (gz - half) * BLADE_SPACING + jitter_z,
    );

    let orientation = rand(instance_index, 3u) * 2.0 * PI;
    let height = mix(BLADE_HEIGHT_MIN, BLADE_HEIGHT_MAX, rand(instance_index, 4u));
    let phase = rand(instance_index, 5u) * 2.0 * PI;
    let lean = (rand(instance_index, 6u) - 0.5) * 0.25;

    // --- walk the triangle strip: two vertices per segment, tapering to a tip ---
    let seg = vertex_index / 2u;
    let side = f32(vertex_index % 2u) * 2.0 - 1.0; // -1 left edge, +1 right edge
    let t = f32(seg) / f32(SEGMENTS);              // 0 at the base .. 1 at the tip
    let width = BLADE_WIDTH * (1.0 - t);           // taper to a point

    // Wind: layered sines that scroll across the field over time.
    let wind_phase = grass.time * grass.wind_speed + base.x * 0.6 + base.z * 0.6 + phase;
    let sway = (sin(wind_phase) + 0.4 * sin(wind_phase * 2.3)) * grass.wind_strength;

    let facing = vec3<f32>(cos(orientation), 0.0, sin(orientation));
    let across = vec3<f32>(-facing.z, 0.0, facing.x);

    let bend = (lean + sway) * t * t;              // bend grows toward the tip
    let local = base
        + across * (side * width)
        + vec3<f32>(0.0, t * height, 0.0)
        + facing * (bend * height);

    // Normal: the blade faces along `facing`, tilted up a touch.
    let normal = normalize(facing - vec3<f32>(0.0, bend, 0.0) + vec3<f32>(0.0, 0.35, 0.0));

    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(local, 1.0);
    out.height_t = t;
    out.world_normal = normal;
    return out;
}

const SUN_DIR: vec3<f32> = vec3<f32>(0.40, 0.82, 0.41);
const BASE_COLOR: vec3<f32> = vec3<f32>(0.05, 0.18, 0.04);
const TIP_COLOR: vec3<f32> = vec3<f32>(0.45, 0.62, 0.18);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sun = normalize(SUN_DIR);
    let n = normalize(in.world_normal);
    let wrap = max(dot(n, sun), 0.0) * 0.6 + 0.4;  // soft wrap lighting

    let albedo = mix(BASE_COLOR, TIP_COLOR, in.height_t);
    let ao = mix(0.55, 1.0, in.height_t);          // darker down at the ground
    return vec4<f32>(albedo * wrap * ao, 1.0);
}
