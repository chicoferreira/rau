// Rectangular area lights, shaded with Linearly Transformed Cosines.
//
// `integrate_edge_vec`, `ltc_evaluate` and the lookup tables are ported from the
// reference implementation of:
//
//   Real-Time Polygonal-Light Shading with Linearly Transformed Cosines.
//   Eric Heitz, Jonathan Dupuy, Stephen Hill and David Neubelt.
//   ACM Transactions on Graphics (Proceedings of ACM SIGGRAPH 2016) 35(4), 2016.
//   https://eheitzresearch.wordpress.com/415-2/
//
// Reference code (c) 2017 Heitz, Dupuy, Hill and Neubelt (BSD-3-clause, citation
// required): https://github.com/selfshadow/ltc_code — via the WGSL-adapted GLSL
// in https://learnopengl.com/Guest-Articles/2022/Area-Lights

const LIGHT_COUNT: u32 = 3u;

// The tables are 64x64, sampled half a texel in from the edge.
const LUT_SCALE: f32 = 63.0 / 64.0;
const LUT_BIAS: f32 = 0.5 / 64.0;

struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}

// The transform maps a unit quad in the XY plane onto the panel: scale is its
// width and height, translation its centre.
struct AreaLight {
    transform: mat4x4<f32>,
    color: vec4<f32>,
    intensity: f32,
}

struct Lights {
    lights: array<AreaLight, LIGHT_COUNT>,
}

struct Material {
    floor_albedo: vec4<f32>,
    wall_albedo: vec4<f32>,
    floor_roughness: f32,
    wall_roughness: f32,
    specular: f32,
    exposure: f32,
    ambient: f32,
}

@group(0) @binding(0)
var ltc1_texture: texture_2d<f32>;

@group(0) @binding(1)
var ltc2_texture: texture_2d<f32>;

@group(0) @binding(2)
var ltc_sampler: sampler;

@group(1) @binding(0)
var<uniform> camera: Camera;

@group(2) @binding(0)
var<uniform> lights: Lights;

@group(3) @binding(0)
var<uniform> material: Material;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) @interpolate(flat) is_floor: u32,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    // A box with its normals facing inwards: floor, ceiling, back, left, right.
    var origins = array<vec3<f32>, 5>(
        vec3<f32>(-4.0, 0.0, -3.0),
        vec3<f32>(-4.0, 3.2, 3.0),
        vec3<f32>(-4.0, 0.0, -3.0),
        vec3<f32>(-4.0, 0.0, 3.0),
        vec3<f32>(4.0, 0.0, -3.0),
    );
    var edge_u = array<vec3<f32>, 5>(
        vec3<f32>(8.0, 0.0, 0.0),
        vec3<f32>(8.0, 0.0, 0.0),
        vec3<f32>(8.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, -6.0),
        vec3<f32>(0.0, 0.0, 6.0),
    );
    var edge_v = array<vec3<f32>, 5>(
        vec3<f32>(0.0, 0.0, 6.0),
        vec3<f32>(0.0, 0.0, -6.0),
        vec3<f32>(0.0, 3.2, 0.0),
        vec3<f32>(0.0, 3.2, 0.0),
        vec3<f32>(0.0, 3.2, 0.0),
    );
    var normals = array<vec3<f32>, 5>(
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, -1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(-1.0, 0.0, 0.0),
    );
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );

    let quad = index / 6u;
    let corner = corners[index % 6u];
    let world_position = origins[quad] + edge_u[quad] * corner.x + edge_v[quad] * corner.y;

    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    out.world_normal = normals[quad];
    out.is_floor = select(0u, 1u, quad == 0u);
    return out;
}

struct LightQuad {
    p0: vec3<f32>,
    p1: vec3<f32>,
    p2: vec3<f32>,
    p3: vec3<f32>,
}

// The same unit quad panel.wgsl draws, put through the same transform, so the
// rectangle being integrated is the one on screen.
fn light_quad(light: AreaLight) -> LightQuad {
    var quad: LightQuad;
    quad.p0 = (light.transform * vec4<f32>(-0.5, -0.5, 0.0, 1.0)).xyz;
    quad.p1 = (light.transform * vec4<f32>(0.5, -0.5, 0.0, 1.0)).xyz;
    quad.p2 = (light.transform * vec4<f32>(0.5, 0.5, 0.0, 1.0)).xyz;
    quad.p3 = (light.transform * vec4<f32>(-0.5, 0.5, 0.0, 1.0)).xyz;
    return quad;
}

// The edge integral, as a vector so it can also be used for sphere clipping.
// The built-in acos() is not accurate enough here, so this is the fitted
// approximation from the reference implementation.
fn integrate_edge_vec(v1: vec3<f32>, v2: vec3<f32>) -> vec3<f32> {
    let x = dot(v1, v2);
    let y = abs(x);

    let a = 0.8543985 + (0.4965155 + 0.0145206 * y) * y;
    let b = 3.4175940 + (4.1616724 + y) * y;
    let v = a / b;

    var theta_sintheta: f32;
    if x > 0.0 {
        theta_sintheta = v;
    } else {
        theta_sintheta = 0.5 * inverseSqrt(max(1.0 - x * x, 1e-7)) - v;
    }

    return cross(v1, v2) * theta_sintheta;
}

// How much light the quad delivers to `p`. `minv` warps the cosine distribution
// into the shape of the BRDF lobe; passing the identity gives the diffuse term.
fn ltc_evaluate(
    n: vec3<f32>,
    v: vec3<f32>,
    p: vec3<f32>,
    minv: mat3x3<f32>,
    quad: LightQuad,
) -> f32 {
    // Orthonormal basis around the normal.
    let t1 = normalize(v - n * dot(v, n));
    let t2 = cross(n, t1);
    let transform = minv * transpose(mat3x3<f32>(t1, t2, n));

    // Pull the polygon back into the plain cosine-weighted space.
    let l0 = normalize(transform * (quad.p0 - p));
    let l1 = normalize(transform * (quad.p1 - p));
    let l2 = normalize(transform * (quad.p2 - p));
    let l3 = normalize(transform * (quad.p3 - p));

    var vsum = vec3<f32>(0.0);
    vsum += integrate_edge_vec(l0, l1);
    vsum += integrate_edge_vec(l1, l2);
    vsum += integrate_edge_vec(l2, l3);
    vsum += integrate_edge_vec(l3, l0);

    let len = length(vsum);

    // Which side of the panel we are on decides how the horizon cuts the lobe.
    let light_normal = cross(quad.p1 - quad.p0, quad.p3 - quad.p0);
    let facing = dot(quad.p0 - p, light_normal);

    var z = vsum.z / len;
    if facing < 0.0 {
        z = -z;
    }

    var uv = vec2<f32>(z * 0.5 + 0.5, len);
    uv = uv * LUT_SCALE + LUT_BIAS;

    // The tabulated horizon-clipped sphere corrects the form factor.
    let scale = textureSampleLevel(ltc2_texture, ltc_sampler, uv, 0.0).w;
    return len * scale;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let is_floor = in.is_floor == 1u;

    // Grout lines every metre, darker and rougher than the tiles. Computed for
    // every fragment and then masked, because derivatives need uniform control
    // flow. Without something for the reflections to cross, the floor reads as
    // flat paint.
    let tile = abs(fract(in.world_position.xz - 0.5) - 0.5);
    let width = max(fwidth(in.world_position.xz), vec2<f32>(1e-5));
    let edge = min(min(tile.x / width.x, tile.y / width.y), 1.0);
    let grout = (1.0 - edge) * f32(in.is_floor);

    let albedo = select(material.wall_albedo.rgb, material.floor_albedo.rgb, is_floor)
        * mix(1.0, 0.35, grout);
    let roughness = select(material.wall_roughness, material.floor_roughness, is_floor)
        + grout * 0.25;

    let n = normalize(in.world_normal);
    let v = normalize(camera.position.xyz - in.world_position);
    let p = in.world_position;
    let dot_nv = clamp(dot(n, v), 0.0, 1.0);

    // Both tables are indexed by roughness and viewing angle.
    var uv = vec2<f32>(roughness, sqrt(1.0 - dot_nv));
    uv = uv * LUT_SCALE + LUT_BIAS;

    // t1: the four non-zero entries of the inverse LTC matrix.
    // t2: GGX norm, Fresnel, unused, sphere form factor.
    let t1 = textureSampleLevel(ltc1_texture, ltc_sampler, uv, 0.0);
    let t2 = textureSampleLevel(ltc2_texture, ltc_sampler, uv, 0.0);

    let minv = mat3x3<f32>(
        vec3<f32>(t1.x, 0.0, t1.y),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(t1.z, 0.0, t1.w),
    );
    let identity = mat3x3<f32>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
    );

    let specular_color = vec3<f32>(material.specular);
    let fresnel = specular_color * t2.x + (vec3<f32>(1.0) - specular_color) * t2.y;

    var result = albedo * material.ambient;
    for (var i = 0u; i < LIGHT_COUNT; i = i + 1u) {
        let light = lights.lights[i];
        let quad = light_quad(light);

        let diffuse = ltc_evaluate(n, v, p, identity, quad);
        let specular = ltc_evaluate(n, v, p, minv, quad);

        result += light.color.rgb * light.intensity * (specular * fresnel + albedo * diffuse);
    }

    // The render target is sRGB, so the hardware does the encode and this stays
    // linear.
    let mapped = vec3<f32>(1.0) - exp(-result * material.exposure);
    return vec4<f32>(mapped, 1.0);
}
