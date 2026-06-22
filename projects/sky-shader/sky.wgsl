// Preetham analytic sky model, ported from Embark Studios' rust-gpu sky-shader
// example (dual MIT/Apache-2.0):
// https://github.com/EmbarkStudios/rust-gpu/blob/main/examples/shaders/sky-shader/src/lib.rs
//
// The original computes the view ray from the fragment coordinate. Here we drive
// the ray direction from the scene camera (inverse projection + inverse view) so
// the sky reacts to the camera bound to the viewport.

struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Sky {
    sun_position: vec3<f32>,
}
@group(1) @binding(0)
var<uniform> sky_uniform: Sky;

const PI: f32 = 3.141592653589793;

// Atmospheric parameters.
const DEPOLARIZATION_FACTOR: f32 = 0.035;
const MIE_COEFFICIENT: f32 = 0.005;
const MIE_DIRECTIONAL_G: f32 = 0.8;
const MIE_K_COEFFICIENT: vec3<f32> = vec3<f32>(0.686, 0.678, 0.666);
const MIE_V: f32 = 4.0;
const MIE_ZENITH_LENGTH: f32 = 1.25e3;
const NUM_MOLECULES: f32 = 2.542e25;
const PRIMARIES: vec3<f32> = vec3<f32>(6.8e-7, 5.5e-7, 4.5e-7);
const RAYLEIGH: f32 = 1.0;
const RAYLEIGH_ZENITH_LENGTH: f32 = 8.4e3;
const REFRACTIVE_INDEX: f32 = 1.0003;
const SUN_ANGULAR_DIAMETER_DEGREES: f32 = 0.0093333;
const SUN_INTENSITY_FACTOR: f32 = 1000.0;
const SUN_INTENSITY_FALLOFF_STEEPNESS: f32 = 1.5;
const TURBIDITY: f32 = 2.0;

// Replaces the rust-gpu specialization constant `sun_intensity_extra_spec_const_factor / 100`.
const SUN_INTENSITY_EXTRA_FACTOR: f32 = 1.0;

// Tonemap coefficients.
const TONEMAP_A: f32 = 2.35;
const TONEMAP_B: f32 = 2.8826666;
const TONEMAP_C: f32 = 789.7459;
const TONEMAP_D: f32 = 0.935;

fn tonemap(col: vec3<f32>) -> vec3<f32> {
    let z = pow(col, vec3<f32>(TONEMAP_A));
    return z / (pow(z, vec3<f32>(TONEMAP_D)) * TONEMAP_B + vec3<f32>(TONEMAP_C));
}

fn total_rayleigh(lambda: vec3<f32>) -> vec3<f32> {
    return (8.0 * pow(PI, 3.0)
        * pow(pow(REFRACTIVE_INDEX, 2.0) - 1.0, 2.0)
        * (6.0 + 3.0 * DEPOLARIZATION_FACTOR))
        / (3.0 * NUM_MOLECULES * pow(lambda, vec3<f32>(4.0))
            * (6.0 - 7.0 * DEPOLARIZATION_FACTOR));
}

fn total_mie(lambda: vec3<f32>, k: vec3<f32>, t: f32) -> vec3<f32> {
    let c = 0.2 * t * 10e-18;
    return 0.434 * c * PI * pow((2.0 * PI) / lambda, vec3<f32>(MIE_V - 2.0)) * k;
}

fn rayleigh_phase(cos_theta: f32) -> f32 {
    return (3.0 / (16.0 * PI)) * (1.0 + pow(cos_theta, 2.0));
}

fn henyey_greenstein_phase(cos_theta: f32, g: f32) -> f32 {
    return (1.0 / (4.0 * PI))
        * ((1.0 - pow(g, 2.0))
            / pow(1.0 - 2.0 * g * cos_theta + pow(g, 2.0), 1.5));
}

fn sun_intensity(zenith_angle_cos: f32) -> f32 {
    let cutoff_angle = PI / 1.95;
    return SUN_INTENSITY_FACTOR
        * max(0.0,
        1.0 - exp(-((cutoff_angle - acos(zenith_angle_cos)) / SUN_INTENSITY_FALLOFF_STEEPNESS)));
}

fn sky(dir: vec3<f32>, sun_position: vec3<f32>) -> vec3<f32> {
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let sunfade = 1.0 - (1.0 - exp(saturate(sun_position.y / 450000.0)));
    let rayleigh_coefficient = RAYLEIGH - (1.0 * (1.0 - sunfade));
    let beta_r = total_rayleigh(PRIMARIES) * rayleigh_coefficient;

    let beta_m = total_mie(PRIMARIES, MIE_K_COEFFICIENT, TURBIDITY) * MIE_COEFFICIENT;

    let zenith_angle = acos(max(dot(up, dir), 0.0));
    let denom = cos(zenith_angle)
        + 0.15 * pow(93.885 - ((zenith_angle * 180.0) / PI), -1.253);

    let s_r = RAYLEIGH_ZENITH_LENGTH / denom;
    let s_m = MIE_ZENITH_LENGTH / denom;

    let fex = exp(-(beta_r * s_r + beta_m * s_m));

    let sun_direction = normalize(sun_position);
    let cos_theta = dot(dir, sun_direction);
    let beta_r_theta = beta_r * rayleigh_phase(cos_theta * 0.5 + 0.5);

    let beta_m_theta = beta_m * henyey_greenstein_phase(cos_theta, MIE_DIRECTIONAL_G);
    let sun_e = sun_intensity(dot(sun_direction, up)) * SUN_INTENSITY_EXTRA_FACTOR;

    var lin = pow(
        sun_e * ((beta_r_theta + beta_m_theta) / (beta_r + beta_m)) * (vec3<f32>(1.0) - fex),
        vec3<f32>(1.5),
    );

    lin *= mix(
        vec3<f32>(1.0),
        pow(sun_e * ((beta_r_theta + beta_m_theta) / (beta_r + beta_m)) * fex, vec3<f32>(0.5)),
        saturate(pow(1.0 - dot(up, sun_direction), 5.0)),
    );

    let sun_angular_diameter_cos = cos(SUN_ANGULAR_DIAMETER_DEGREES);
    let sundisk = smoothstep(
        sun_angular_diameter_cos,
        sun_angular_diameter_cos + 0.00002,
        cos_theta,
    );
    var l0 = 0.1 * fex;
    l0 += sun_e * 19000.0 * fex * sundisk;

    return lin + l0;
}

struct VertexOutput {
    @builtin(position) frag_position: vec4<f32>,
    @location(0) clip_position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
    let uv = vec2<f32>(vec2<u32>(
        id & 1u,
        (id >> 1u) & 1u,
    ));
    var out: VertexOutput;
    out.clip_position = vec4(uv * 4.0 - 1.0, 1.0, 1.0);
    out.frag_position = out.clip_position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_pos_homogeneous = camera.inv_proj * in.clip_position;
    let view_ray_direction = view_pos_homogeneous.xyz / view_pos_homogeneous.w;
    let ray_direction = normalize((camera.inv_view * vec4(view_ray_direction, 0.0)).xyz);

    var color = sky(ray_direction, sky_uniform.sun_position);
    color = min(max(color, vec3<f32>(0.0)), vec3<f32>(1024.0));

    return vec4<f32>(tonemap(color), 1.0);
}
