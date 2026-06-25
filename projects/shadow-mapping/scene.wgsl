// Scene pass: shade the boxes from the camera and decide, per fragment, whether
// it sits in shadow by reprojecting into light space and comparing against the
// shadow map written in the previous pass.
//
// The `object` table (and its length) must stay in sync with `shadow.wgsl` and
// with src/scene/shadow_mapping.rs, which derives the instanced draw count.

struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct Light {
    projection_view: mat4x4<f32>,
    position: vec4<f32>,
    color: vec3<f32>,
}
@group(1) @binding(0)
var<uniform> light: Light;

@group(2) @binding(0)
var shadow_map: texture_depth_2d;

struct Object {
    center: vec3<f32>,
    scale: vec3<f32>,
    color: vec3<f32>,
}

// The boxes, one per instance: index 0 is the flattened floor, the rest are the
// shadow casters. center, scale and color for each on a single row.
fn object(i: u32) -> Object {
    var objects = array<Object, 5>(
        Object(vec3<f32>( 0.0, -0.25,  0.0), vec3<f32>(14.0, 0.5, 14.0), vec3<f32>(0.62, 0.62, 0.65)),
        Object(vec3<f32>(-2.0,  0.75, -0.5), vec3<f32>( 1.5, 1.5,  1.5), vec3<f32>(0.85, 0.35, 0.30)),
        Object(vec3<f32>( 1.8,  0.50,  1.2), vec3<f32>( 1.0, 1.0,  1.0), vec3<f32>(0.30, 0.55, 0.85)),
        Object(vec3<f32>( 0.2,  1.25, -2.0), vec3<f32>( 1.0, 1.0,  1.0), vec3<f32>(0.95, 0.72, 0.25)),
        Object(vec3<f32>( 2.6,  1.00, -1.8), vec3<f32>( 1.0, 2.0,  1.0), vec3<f32>(0.42, 0.75, 0.45)),
    );
    return objects[i];
}

// Unit cube, half-extent 0.5: 6 faces, 2 triangles each.
fn cube_position(vertex_index: u32) -> vec3<f32> {
    var p = array<vec3<f32>, 36>(
        // +X
        vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5),
        vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5), vec3<f32>( 0.5, -0.5,  0.5),
        // -X
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(-0.5,  0.5, -0.5), vec3<f32>(-0.5,  0.5,  0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(-0.5,  0.5,  0.5), vec3<f32>(-0.5, -0.5,  0.5),
        // +Y
        vec3<f32>(-0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5),
        vec3<f32>(-0.5,  0.5, -0.5), vec3<f32>( 0.5,  0.5,  0.5), vec3<f32>(-0.5,  0.5,  0.5),
        // -Y
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5,  0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5,  0.5), vec3<f32>(-0.5, -0.5,  0.5),
        // +Z
        vec3<f32>(-0.5, -0.5,  0.5), vec3<f32>( 0.5, -0.5,  0.5), vec3<f32>( 0.5,  0.5,  0.5),
        vec3<f32>(-0.5, -0.5,  0.5), vec3<f32>( 0.5,  0.5,  0.5), vec3<f32>(-0.5,  0.5,  0.5),
        // -Z
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5),
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>( 0.5,  0.5, -0.5), vec3<f32>(-0.5,  0.5, -0.5),
    );
    return p[vertex_index];
}

// Face normals; axis-aligned box scaling preserves their direction.
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
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let obj = object(instance_index);
    let world = cube_position(vertex_index) * obj.scale + obj.center;

    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(world, 1.0);
    out.world_position = world;
    out.world_normal = cube_normal(vertex_index / 6u);
    out.color = obj.color;
    return out;
}

// How shadowed `world_position` is, in [0, 1]. 0 = fully lit, 1 = fully occluded.
fn shadow_factor(world_position: vec3<f32>, n: vec3<f32>, l: vec3<f32>) -> f32 {
    let light_clip = light.projection_view * vec4<f32>(world_position, 1.0);
    // Behind the light: treat as lit.
    if (light_clip.w <= 0.0) {
        return 0.0;
    }

    let ndc = light_clip.xyz / light_clip.w;
    // NDC xy [-1, 1] -> uv [0, 1], flipping y for texture space.
    var uv = ndc.xy * 0.5 + vec2<f32>(0.5);
    uv.y = 1.0 - uv.y;

    // Outside the light frustum (or past the far plane): nothing to occlude it.
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z > 1.0) {
        return 0.0;
    }

    // wgpu depth is already in [0, 1], so ndc.z is the fragment's light-space depth.
    let current_depth = ndc.z;
    // Slope-scaled bias: steeper surfaces (grazing the light) need more.
    let bias = max(0.0025 * (1.0 - dot(n, l)), 0.0006);

    let dims = vec2<f32>(textureDimensions(shadow_map));
    let max_texel = vec2<i32>(dims) - vec2<i32>(1, 1);
    let base = vec2<i32>(uv * dims);

    // 3x3 percentage-closer filtering to soften the shadow edge.
    var shadow = 0.0;
    for (var x = -1; x <= 1; x = x + 1) {
        for (var y = -1; y <= 1; y = y + 1) {
            let coord = clamp(base + vec2<i32>(x, y), vec2<i32>(0, 0), max_texel);
            let stored = textureLoad(shadow_map, coord, 0);
            shadow += select(0.0, 1.0, current_depth - bias > stored);
        }
    }
    return shadow / 9.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let l = normalize(light.position.xyz - in.world_position);

    let diffuse = max(dot(n, l), 0.0);
    let shadow = shadow_factor(in.world_position, n, l);

    let ambient = 0.18;
    let lit = ambient + (1.0 - shadow) * diffuse * 0.9;
    return vec4<f32>(in.color * light.color * lit, 1.0);
}
