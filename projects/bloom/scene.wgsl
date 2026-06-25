struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) clip_position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> time: f32;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    out.clip_position = vec4<f32>(out.uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

fn orb(uv: vec2<f32>, center: vec2<f32>, radius: f32, color: vec3<f32>, brightness: f32) -> vec3<f32> {
    let d = length(uv - center);
    let core = smoothstep(radius, radius * 0.4, d);
    let inv_glow = radius / (d + 0.002);
    let glow = pow(inv_glow, 2.5) * 0.03;
    return color * brightness * (core + glow);
}

fn hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

fn stars(uv: vec2<f32>) -> vec3<f32> {
    let grid = floor(uv * 40.0);
    let f = fract(uv * 40.0);
    let h = hash(grid);
    let center = vec2<f32>(hash(grid + 0.1), hash(grid + 0.3));
    let d = length(f - center);
    let brightness = step(0.92, h) * smoothstep(0.05, 0.0, d) * (0.3 + 0.7 * h);
    return vec3<f32>(brightness * 0.4);
}

@fragment
fn fs_main(vs: VertexOutput) -> @location(0) vec4<f32> {
    let uv = vs.uv;
    let t = time;

    var color = vec3<f32>(0.005, 0.005, 0.02);

    color += stars(uv);

    let pulse = 0.85 + 0.15 * sin(t * 1.2);
    color += orb(uv, vec2<f32>(0.5, 0.5), 0.055, vec3<f32>(1.0, 0.92, 0.7), 5.0 * pulse);

    let a1 = t * 0.4;
    let p1 = vec2<f32>(0.5 + cos(a1) * 0.25, 0.5 + sin(a1) * 0.22);
    color += orb(uv, p1, 0.028, vec3<f32>(1.0, 0.15, 0.45), 4.5);

    let a2 = t * -0.3 + 2.094;
    let p2 = vec2<f32>(0.5 + cos(a2) * 0.30, 0.5 + sin(a2) * 0.28);
    color += orb(uv, p2, 0.024, vec3<f32>(0.15, 0.75, 1.0), 6.0);

    let p3 = vec2<f32>(0.2, 0.5 + sin(t * 0.7) * 0.12);
    let pulse3 = 0.7 + 0.3 * sin(t * 1.8 + 1.0);
    color += orb(uv, p3, 0.032, vec3<f32>(1.0, 0.55, 0.05), 3.5 * pulse3);

    let p4 = vec2<f32>(0.78, 0.35 + sin(t * 0.5 + 3.0) * 0.08);
    color += orb(uv, p4, 0.020, vec3<f32>(0.3, 1.0, 0.4), 5.5);

    let a5 = t * 0.6 + 4.2;
    let p5 = vec2<f32>(0.5 + cos(a5) * 0.18, 0.5 + sin(a5) * 0.18);
    let pulse5 = 0.6 + 0.4 * sin(t * 2.5);
    color += orb(uv, p5, 0.018, vec3<f32>(0.7, 0.2, 1.0), 7.0 * pulse5);

    let p6 = vec2<f32>(0.82, 0.72);
    color += orb(uv, p6, 0.022, vec3<f32>(1.0, 0.85, 0.3), 3.0);

    return vec4<f32>(color, 1.0);
}
