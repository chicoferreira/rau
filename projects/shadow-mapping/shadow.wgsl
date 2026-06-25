// Shadow pass: render the same boxes from the light's point of view. The depth
// buffer this produces is the shadow map sampled by `scene.wgsl`. The colour
// output is a simple shaded "what the light sees" image, displayed in the light
// viewport — everything the light can see is lit, so no shadow lookup is needed.
//
// The `object` table must match `scene.wgsl` and src/scene/shadow_mapping.rs.
// The geometry here is identical to the scene pass; only the transform (light
// instead of camera) and the absence of a shadow term differ.

struct Light {
    projection_view: mat4x4<f32>,
    position: vec4<f32>,
    color: vec3<f32>,
}
@group(0) @binding(0)
var<uniform> light: Light;

struct Object {
    center: vec3<f32>,
    scale: vec3<f32>,
    color: vec3<f32>,
}

// The boxes, one per instance: index 0 is the flattened floor, the rest are the
// shadow casters. Kept identical to `scene.wgsl`.
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

// Unit cube, half-extent 0.5: 6 faces, 2 triangles each. Identical to scene.wgsl.
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
    out.clip_position = light.projection_view * vec4<f32>(world, 1.0);
    out.world_position = world;
    out.world_normal = cube_normal(vertex_index / 6u);
    out.color = obj.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Everything the light can see is, by definition, lit — so this is plain
    // diffuse shading with no shadow term. The depth that backs this image is
    // what `scene.wgsl` reads as the shadow map.
    let n = normalize(in.world_normal);
    let l = normalize(light.position.xyz - in.world_position);

    let diffuse = max(dot(n, l), 0.0);
    let lit = 0.18 + diffuse * 0.9;
    return vec4<f32>(in.color * light.color * lit, 1.0);
}
