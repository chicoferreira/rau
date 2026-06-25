// SSAO pass: estimate per-pixel ambient occlusion from the view-space position
// and normal G-buffers. For each fragment we build a TBN frame (the normal plus
// a randomly rotated tangent), scatter a hemisphere of sample points around the
// fragment, project each one back to the screen, and check whether stored
// geometry sits in front of it — if so, that direction is occluded.
//
// Ported from https://learnopengl.com/Advanced-Lighting/SSAO (CC BY-NC 4.0).
// Unlike the tutorial, the hemisphere kernel and the rotation noise are NOT
// uploaded from the CPU (rau uniforms have no array type): both are generated
// procedurally below with hash functions, so the effect is fully GPU-resident.

struct Camera {
    position: vec4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    projection_view: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0) var g_position: texture_2d<f32>;
@group(1) @binding(1) var g_normal: texture_2d<f32>;
@group(1) @binding(2) var g_sampler: sampler;

struct SsaoParams {
    radius: f32,
    bias: f32,
    power: f32,
    kernel_size: u32,
}
@group(2) @binding(0)
var<uniform> params: SsaoParams;

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

// --- Hash helpers (Dave Hoskins, https://www.shadertoy.com/view/4djSRW). ---
fn hash11(p: f32) -> f32 {
    var x = fract(p * 0.1031);
    x = x * (x + 33.33);
    x = x * (x + x);
    return fract(x);
}

fn hash33(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx);
}

// One hemisphere sample (tangent space, +z away from the surface). Samples are
// packed towards the origin so nearby geometry dominates the occlusion, exactly
// like the `lerp(0.1, 1.0, scale*scale)` weighting in the tutorial.
fn kernel_sample(i: u32, count: u32) -> vec3<f32> {
    let fi = f32(i);
    let r = hash33(vec3<f32>(fi, fi * 0.37 + 1.0, fi * 1.71 + 2.0));
    var s = normalize(vec3<f32>(r.x * 2.0 - 1.0, r.y * 2.0 - 1.0, r.z + 0.05));
    s = s * hash11(fi * 2.13 + 0.5); // random radius in [0, 1)
    var scale = fi / f32(count);
    scale = mix(0.1, 1.0, scale * scale);
    return s * scale;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let frag_pos = textureSample(g_position, g_sampler, in.uv).xyz;
    let normal = normalize(textureSample(g_normal, g_sampler, in.uv).xyz);

    // Background (no geometry was written here): fully unoccluded.
    if (dot(normal, normal) < 0.5) {
        return vec4<f32>(1.0);
    }

    // Per-pixel random rotation, tiled over 4x4 like the tutorial's noise texture
    // so the following 4x4 blur cancels it cleanly.
    let pixel = vec2<f32>(floor(in.clip_position.xy)) % 4.0;
    let rnd = hash33(vec3<f32>(pixel, 7.0));
    let random_vec = normalize(vec3<f32>(rnd.x * 2.0 - 1.0, rnd.y * 2.0 - 1.0, 0.0));

    // Gram-Schmidt: an orthonormal TBN frame around the surface normal.
    let tangent = normalize(random_vec - normal * dot(random_vec, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, normal);

    var occlusion = 0.0;
    for (var i = 0u; i < params.kernel_size; i = i + 1u) {
        // Sample position in view space.
        let sample_pos = frag_pos + (tbn * kernel_sample(i, params.kernel_size)) * params.radius;

        // Project it to screen space to look up the geometry actually drawn there.
        var offset = camera.projection * vec4<f32>(sample_pos, 1.0);
        let ndc = offset.xy / offset.w;
        let sample_uv = vec2<f32>(ndc.x * 0.5 + 0.5, 1.0 - (ndc.y * 0.5 + 0.5));

        let sample_depth = textureSample(g_position, g_sampler, sample_uv).z;

        // Only count occluders within `radius`, fading out at the edge so distant
        // background geometry doesn't bleed dark halos onto foreground objects.
        let range_check = smoothstep(0.0, 1.0, params.radius / abs(frag_pos.z - sample_depth));
        occlusion = occlusion + select(0.0, 1.0, sample_depth >= sample_pos.z + params.bias) * range_check;
    }

    occlusion = 1.0 - occlusion / f32(params.kernel_size);
    occlusion = pow(clamp(occlusion, 0.0, 1.0), params.power);
    return vec4<f32>(occlusion, occlusion, occlusion, 1.0);
}
