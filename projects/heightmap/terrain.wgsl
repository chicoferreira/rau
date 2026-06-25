const GRID_SIZE: u32 = 256u;
const QUAD_COUNT: u32 = GRID_SIZE - 1u;

struct CameraUniform {
    position: vec3<f32>,
    projection_view: mat4x4<f32>,
};

struct TerrainSettings {
    height_scale: f32,
    terrain_size: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var heightmap: texture_2d<f32>;
@group(1) @binding(1)
var tex_sampler: sampler;
@group(1) @binding(2)
var<uniform> terrain: TerrainSettings;

fn sample_height(uv: vec2<f32>) -> f32 {
    return textureSampleLevel(heightmap, tex_sampler, uv, 0.0).r;
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    let quad_idx = vi / 6u;
    let vert_in_quad = vi % 6u;

    let quad_x = quad_idx % QUAD_COUNT;
    let quad_y = quad_idx / QUAD_COUNT;

    var dx: u32;
    var dy: u32;
    switch vert_in_quad {
        case 0u { dx = 0u; dy = 0u; }
        case 1u { dx = 0u; dy = 1u; }
        case 2u { dx = 1u; dy = 0u; }
        case 3u { dx = 1u; dy = 0u; }
        case 4u { dx = 0u; dy = 1u; }
        case 5u { dx = 1u; dy = 1u; }
        default { dx = 0u; dy = 0u; }
    }

    let gx = quad_x + dx;
    let gy = quad_y + dy;
    let uv = vec2<f32>(f32(gx), f32(gy)) / f32(GRID_SIZE - 1u);

    let h = sample_height(uv);
    let half_size = terrain.terrain_size * 0.5;
    let world_pos = vec3<f32>(
        (uv.x - 0.5) * terrain.terrain_size,
        h * terrain.height_scale,
        (uv.y - 0.5) * terrain.terrain_size,
    );

    let texel = 1.0 / f32(GRID_SIZE - 1u);
    let hL = sample_height(uv + vec2<f32>(-texel, 0.0));
    let hR = sample_height(uv + vec2<f32>(texel, 0.0));
    let hD = sample_height(uv + vec2<f32>(0.0, -texel));
    let hU = sample_height(uv + vec2<f32>(0.0, texel));

    let cell_size = terrain.terrain_size / f32(GRID_SIZE - 1u);
    let dhdx = (hR - hL) * terrain.height_scale / (2.0 * cell_size);
    let dhdz = (hU - hD) * terrain.height_scale / (2.0 * cell_size);
    let normal = normalize(vec3<f32>(-dhdx, 1.0, -dhdz));

    var out: VertexOutput;
    out.clip_position = camera.projection_view * vec4<f32>(world_pos, 1.0);
    out.normal = normal;
    out.world_pos = world_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let slope = 1.0 - dot(n, vec3<f32>(0.0, 1.0, 0.0));

    let grass = vec3<f32>(0.15, 0.45, 0.1);
    let rock = vec3<f32>(0.45, 0.42, 0.38);
    let color = mix(grass, rock, smoothstep(0.25, 0.55, slope));

    let sun_dir = normalize(vec3<f32>(0.4, 0.8, 0.3));
    let diffuse = max(dot(n, sun_dir), 0.0);
    let lit = color * (0.25 + diffuse * 0.75);

    return vec4<f32>(lit, 1.0);
}
