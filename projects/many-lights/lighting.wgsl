const PI: f32 = 3.14159265;
const NUM_LIGHTS: u32 = 256u;
const FIELD_HALF: f32 = 11.0;
const ALBEDO: vec3<f32> = vec3<f32>(0.45, 0.45, 0.45);

struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct SceneSettings {
    time: f32,
    light_radius: f32,
    ambient: f32,
}
@group(1) @binding(0)
var<uniform> scene: SceneSettings;

@group(2) @binding(0) var g_position: texture_2d<f32>;
@group(2) @binding(1) var g_normal: texture_2d<f32>;
@group(2) @binding(2) var g_sampler: sampler;

fn hash_f(x: f32) -> f32 { return fract(sin(x * 127.1) * 43758.5453); }
fn hash3(seed: f32) -> vec3<f32> {
    return vec3<f32>(
        hash_f(seed),
        hash_f(seed * 2.3717),
        hash_f(seed * 4.6831),
    );
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let x = c * (1.0 - abs(((h * 6.0) % 2.0) - 1.0));
    let m = v - c;
    let hi = u32(h * 6.0) % 6u;
    var rgb: vec3<f32>;
    switch hi {
        case 0u { rgb = vec3(c, x, 0.0); }
        case 1u { rgb = vec3(x, c, 0.0); }
        case 2u { rgb = vec3(0.0, c, x); }
        case 3u { rgb = vec3(0.0, x, c); }
        case 4u { rgb = vec3(x, 0.0, c); }
        default { rgb = vec3(c, 0.0, x); }
    }
    return rgb + m;
}

fn light_position(i: u32) -> vec3<f32> {
    let seed = f32(i);
    let h = hash3(seed * 1.7);
    let base = vec3<f32>(
        (h.x - 0.5) * FIELD_HALF * 2.0,
        h.y * 2.0 + 0.6,
        (h.z - 0.5) * FIELD_HALF * 2.0,
    );
    let phase = seed * 0.371;
    let t = scene.time;
    let orbit = vec3<f32>(
        sin(t * 0.4 + phase) * 0.8,
        sin(t * 0.6 + phase * 2.3) * 0.3,
        cos(t * 0.35 + phase * 1.7) * 0.8,
    );
    return base + orbit;
}

fn light_color(i: u32) -> vec3<f32> {
    let hue = hash_f(f32(i) * 3.17);
    return hsv_to_rgb(hue, 1.0, 1.0);
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    out.clip_position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let frag_pos = textureSample(g_position, g_sampler, in.uv).xyz;
    let normal = textureSample(g_normal, g_sampler, in.uv).xyz;

    if dot(normal, normal) < 0.5 {
        return vec4<f32>(0.01, 0.01, 0.015, 1.0);
    }

    let n = normalize(normal);
    let view_dir = normalize(camera.position.xyz - frag_pos);
    var result = ALBEDO * scene.ambient;

    for (var i = 0u; i < NUM_LIGHTS; i++) {
        let lp = light_position(i);
        let lc = light_color(i);
        let to_light = lp - frag_pos;
        let dist = length(to_light);

        if dist > scene.light_radius { continue; }

        let ld = to_light / dist;
        let falloff = max(1.0 - dist / scene.light_radius, 0.0);
        let atten = falloff * falloff * falloff;

        let diff = max(dot(n, ld), 0.0);
        let half_dir = normalize(ld + view_dir);
        let spec = pow(max(dot(n, half_dir), 0.0), 32.0) * 0.4;

        result += (ALBEDO * diff + spec) * lc * atten;
    }

    result = result / (result + 1.0);

    return vec4<f32>(result, 1.0);
}
