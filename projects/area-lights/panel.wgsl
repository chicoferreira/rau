// The emissive rectangles — the visible face of each area light.
//
// One quad per light, built from the same transform the shading reads, so a
// panel always sits where its light actually is. Move one in the inspector and
// both follow.

const LIGHT_COUNT: u32 = 3u;

struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}

struct AreaLight {
    transform: mat4x4<f32>,
    color: vec3<f32>,
    intensity: f32,
}

struct Lights {
    lights: array<AreaLight, LIGHT_COUNT>,
}

struct Material {
    floor_albedo: vec3<f32>,
    wall_albedo: vec3<f32>,
    floor_roughness: f32,
    wall_roughness: f32,
    specular: f32,
    exposure: f32,
    ambient: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var<uniform> lights: Lights;

@group(2) @binding(0)
var<uniform> material: Material;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) light_index: u32,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let light_index = index / 6u;
    let transform = lights.lights[light_index].transform;

    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, -0.5),
        vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, 0.5),
    );
    let corner = corners[index % 6u];

    var out: VertexOutput;
    out.clip_position = camera.projection_view * transform * vec4<f32>(corner, 0.0, 1.0);
    out.light_index = light_index;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light = lights.lights[in.light_index];
    let emission = light.color * light.intensity;

    // Same tone map as the room, so the panels sit on the curve with the light
    // they cast instead of clipping to flat white.
    let mapped = vec3<f32>(1.0) - exp(-emission * material.exposure);
    return vec4<f32>(mapped, 1.0);
}
