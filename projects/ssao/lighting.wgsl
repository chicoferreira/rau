// Lighting pass: deferred Blinn-Phong shading of the G-buffer, with the ambient
// term scaled by the (blurred) ambient occlusion. Turn the occlusion off and the
// scene flattens out; turn it on and creases, contact shadows and concavities
// gain the soft darkening SSAO is all about.
//
// Ported from https://learnopengl.com/Advanced-Lighting/SSAO (CC BY-NC 4.0).
// Everything happens in view space, where the G-buffer lives and the camera sits
// at the origin, so the light is moved into view space with the view matrix.

struct Camera {
    position: vec4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    projection_view: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
    linear: f32,
    quadratic: f32,
}
@group(1) @binding(0)
var<uniform> light: Light;

@group(2) @binding(0) var g_position: texture_2d<f32>;
@group(2) @binding(1) var g_normal: texture_2d<f32>;
@group(2) @binding(2) var ssao: texture_2d<f32>;
@group(2) @binding(3) var g_sampler: sampler;

// Flat grey albedo, matching the tutorial — the demo is about occlusion, not
// surface colour.
const ALBEDO: vec3<f32> = vec3<f32>(0.95, 0.95, 0.95);
const BACKGROUND: vec3<f32> = vec3<f32>(0.05, 0.06, 0.08);

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
    let occlusion = textureSample(ssao, g_sampler, in.uv).r;

    // Background pixels carry no geometry.
    if (dot(normal, normal) < 0.5) {
        return vec4<f32>(BACKGROUND, 1.0);
    }

    let n = normalize(normal);
    let light_view_pos = (camera.view * vec4<f32>(light.position, 1.0)).xyz;

    // Ambient, dimmed by occlusion — the whole point of SSAO.
    let ambient = ALBEDO * 0.3 * occlusion;

    let to_light = light_view_pos - frag_pos;
    let dist = length(to_light);
    let light_dir = to_light / dist;

    // Diffuse.
    let diff = max(dot(n, light_dir), 0.0);
    let diffuse = ALBEDO * diff * light.color;

    // Blinn-Phong specular (view direction is -frag_pos in view space).
    let view_dir = normalize(-frag_pos);
    let halfway = normalize(light_dir + view_dir);
    let spec = pow(max(dot(n, halfway), 0.0), 16.0);
    let specular = light.color * spec * 0.2;

    let attenuation = 1.0 / (1.0 + light.linear * dist + light.quadratic * dist * dist);

    let lighting = ambient + (diffuse + specular) * attenuation;
    return vec4<f32>(lighting, 1.0);
}
