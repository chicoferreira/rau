const SHELL_COUNT: u32 = 48u;

struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}

struct FurSettings {
    fur_length: f32,
    density: f32,
    time: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var<uniform> light: Light;

@group(2) @binding(0)
var<uniform> fur: FurSettings;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) layer: f32,
}

@vertex
fn vs_main(
    model: VertexInput,
    @builtin(instance_index) shell: u32,
) -> VertexOutput {
    let layer = f32(shell) / f32(SHELL_COUNT - 1u);
    let n = normalize(model.normal);

    let ws = layer * layer * 0.01;
    let p = model.position;
    let t = fur.time;
    let phase = p.x * 2.1 + p.y * 1.7 + p.z * 2.4;
    let gust = sin(t * 0.8 + phase) + 0.5 * sin(t * 1.9 + phase * 2.3) + 0.25 * sin(t * 3.1 + phase * 4.1);
    let wind = vec3<f32>(
        gust * ws,
        sin(t * 2.3 + phase * 1.3) * ws * 0.2,
        (sin(t * 1.1 + phase * 1.8) + 0.4 * sin(t * 2.7 + phase * 3.5)) * ws,
    );

    let world_pos = model.position + n * layer * fur.fur_length + wind;

    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(world_pos, 1.0);
    out.tex_coords = model.tex_coords;
    out.world_position = world_pos;
    out.normal = n;
    out.layer = layer;
    return out;
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let layer = in.layer;

    if layer > 0.02 {
        let uv_scaled = in.tex_coords * fur.density;
        let cell = floor(uv_scaled);
        let local = fract(uv_scaled) - 0.5;
        let h = hash2(cell);

        let strand_prob = 1.0 - layer * layer;
        if h > strand_prob {
            discard;
        }

        let radius = 0.35 * (1.0 - layer * 0.5);
        if length(local) > radius {
            discard;
        }
    }

    let base_color = vec3<f32>(0.85, 0.85, 0.85);
    let tip_color = vec3<f32>(1.0, 1.0, 1.0);
    let fur_color = mix(base_color, tip_color, layer);

    let ao = mix(0.4, 1.0, layer);

    let n = normalize(in.normal);
    let light_dir = normalize(light.position - in.world_position);
    let diffuse = max(dot(n, light_dir), 0.0);

    let lit = fur_color * ao * (0.2 + diffuse * light.color * 0.8);

    return vec4<f32>(lit, 1.0);
}
