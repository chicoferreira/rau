// Parallax occlusion mapping on a brick quad, in tangent space.
//
// Ported from the LearnOpenGL "Parallax Mapping" chapter by Joey de Vries,
// licensed CC BY-NC 4.0: https://learnopengl.com/Advanced-Lighting/Parallax-Mapping
// The brick diffuse, normal, and displacement maps come from the same chapter.
//
// The quad is six procedural vertices (no vertex buffer): a flat surface in the
// world XY plane facing +Z, with a fixed tangent frame T = +X, B = +Y, N = +Z.
// The vertex shader transforms the camera, light, and fragment positions into
// tangent space so the fragment shader can march the view ray through the
// displacement map without needing a per-fragment TBN matrix.

struct Camera {
    position: vec4<f32>,
    projection_view: mat4x4<f32>,
}

@group(1) @binding(0)
var<uniform> camera: Camera;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}

@group(2) @binding(0)
var<uniform> light: Light;

@group(3) @binding(0)
var<uniform> parallax_height_scale: f32;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tangent_frag_pos: vec3<f32>,
    @location(2) tangent_view_pos: vec3<f32>,
    @location(3) tangent_light_pos: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    // Two triangles forming a 2x2 quad centred on the origin in the XY plane.
    var positions = array<vec3<f32>, 6>(
        vec3<f32>(-1.0, -1.0, 0.0),
        vec3<f32>(1.0, -1.0, 0.0),
        vec3<f32>(1.0, 1.0, 0.0),
        vec3<f32>(-1.0, -1.0, 0.0),
        vec3<f32>(1.0, 1.0, 0.0),
        vec3<f32>(-1.0, 1.0, 0.0),
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );

    let world_position = positions[index];

    let t = vec3<f32>(1.0, 0.0, 0.0);
    let b = vec3<f32>(0.0, -1.0, 0.0);
    let n = vec3<f32>(0.0, 0.0, 1.0);
    let world_to_tangent = transpose(mat3x3<f32>(t, b, n));

    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(world_position, 1.0);
    out.tex_coords = uvs[index];
    out.tangent_frag_pos = world_to_tangent * world_position;
    out.tangent_view_pos = world_to_tangent * camera.position.xyz;
    out.tangent_light_pos = world_to_tangent * light.position;
    return out;
}

@group(0) @binding(0)
var diffuse_texture: texture_2d<f32>;

@group(0) @binding(1)
var normal_texture: texture_2d<f32>;

@group(0) @binding(2)
var depth_texture: texture_2d<f32>;

@group(0) @binding(3)
var material_sampler: sampler;

// Parallax occlusion mapping: march the view ray through the depth map in fixed
// layers until it crosses the surface, then refine the hit by interpolating
// between the last two layers. Grazing views use more layers (up to 32) for
// stability.
fn parallax_mapping(tex_coords: vec2<f32>, view_dir: vec3<f32>) -> vec2<f32> {
    let min_layers = 8.0;
    let max_layers = 32.0;
    let num_layers = mix(max_layers, min_layers, abs(view_dir.z));
    let layer_depth = 1.0 / num_layers;

    // Total UV shift across the whole depth range, divided into per-layer steps.
    let p = view_dir.xy / view_dir.z * parallax_height_scale;
    let delta_tex_coords = p / num_layers;

    var current_layer_depth = 0.0;
    var current_tex_coords = tex_coords;
    var current_depth = textureSampleLevel(depth_texture, material_sampler, current_tex_coords, 0.0).r;

    // Step along the ray until the sampled surface is in front of the ray depth.
    while current_layer_depth < current_depth {
        current_tex_coords -= delta_tex_coords;
        current_depth = textureSampleLevel(depth_texture, material_sampler, current_tex_coords, 0.0).r;
        current_layer_depth += layer_depth;
    }

    // Interpolate between the layer before and after the intersection.
    let prev_tex_coords = current_tex_coords + delta_tex_coords;
    let after_depth = current_depth - current_layer_depth;
    let before_depth = textureSampleLevel(depth_texture, material_sampler, prev_tex_coords, 0.0).r
        - current_layer_depth + layer_depth;
    let weight = after_depth / (after_depth - before_depth);
    return mix(current_tex_coords, prev_tex_coords, weight);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(in.tangent_view_pos - in.tangent_frag_pos);
    let tex_coords = parallax_mapping(in.tex_coords, view_dir);

    // Drop fragments whose displaced coordinates fall off the quad.
    if tex_coords.x > 1.0 || tex_coords.y > 1.0 || tex_coords.x < 0.0 || tex_coords.y < 0.0 {
        discard;
    }

    let albedo = textureSample(diffuse_texture, material_sampler, tex_coords).rgb;
    let normal = normalize(textureSample(normal_texture, material_sampler, tex_coords).xyz * 2.0 - 1.0);

    let light_dir = normalize(in.tangent_light_pos - in.tangent_frag_pos);
    let halfway = normalize(light_dir + view_dir);

    let ambient = 0.1 * albedo;
    let diffuse = max(dot(light_dir, normal), 0.0) * albedo;
    let specular = pow(max(dot(normal, halfway), 0.0), 32.0) * vec3<f32>(0.2);

    let color = (ambient + diffuse + specular) * light.color;
    return vec4<f32>(color, 1.0);
}
