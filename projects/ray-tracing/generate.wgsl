// Bakes the spheres into the scene texture, one column per sphere. Everything
// `trace.wgsl` intersects against is written here.
//
// The scene is the closing one from *Ray Tracing in One Weekend* (CC0):
// https://raytracing.github.io
//
// This runs under the `OnChange` dispatch policy, so it re-dispatches only when
// the scene uniform changes. Editing the seed or widening the grid in the
// inspector rebuilds every sphere in a single dispatch.
//
// The scene is stored in a `Rgba32Float` texture. A storage buffer would model 
// this better, but Rau doesn't support them yet.

// One invocation owns one slot and writes all three of its rows, so no invocation
// reads what another wrote. The rows are a structure of arrays, which lets
// `trace.wgsl`'s search loop touch only row 0:
//
//   row 0: center.xyz, radius       (radius 0 marks an empty slot)
//   row 1: albedo.rgb, fuzz or IOR
//   row 2: material kind
//

@group(0) @binding(0)
var scene: texture_storage_2d<rgba32float, write>;

struct Scene {
    // Width is the sphere capacity, height is the row count above.
    scene_size: vec2<u32>,
    seed: u32,
    grid_extent: u32,
    metal_chance: f32,
    glass_chance: f32,
    small_radius: f32,
    glass_ior: f32,
}
@group(0) @binding(1)
var<uniform> scene_params: Scene;

const KIND_LAMBERTIAN: f32 = 0.0;
const KIND_METAL: f32 = 1.0;
const KIND_DIELECTRIC: f32 = 2.0;

// PCG, a small permuted-congruential generator. `state` is advanced in place so
// successive calls give independent values.
fn next_u32(state: ptr<function, u32>) -> u32 {
    let advanced = (*state) * 747796405u + 2891336453u;
    *state = advanced;
    let word = ((advanced >> ((advanced >> 28u) + 4u)) ^ advanced) * 277803737u;
    return (word >> 22u) ^ word;
}

// Uniform in [0, 1).
fn next_f32(state: ptr<function, u32>) -> f32 {
    return f32(next_u32(state)) * (1.0 / 4294967296.0);
}

fn next_range(state: ptr<function, u32>, low: f32, high: f32) -> f32 {
    return low + (high - low) * next_f32(state);
}

fn store_sphere(
    slot: u32,
    center: vec3<f32>,
    radius: f32,
    albedo: vec3<f32>,
    param: f32,
    kind: f32,
) {
    let column = i32(slot);
    textureStore(scene, vec2<i32>(column, 0), vec4<f32>(center, radius));
    textureStore(scene, vec2<i32>(column, 1), vec4<f32>(albedo, param));
    textureStore(scene, vec2<i32>(column, 2), vec4<f32>(kind, 0.0, 0.0, 0.0));
}

fn store_empty(slot: u32) {
    store_sphere(slot, vec3<f32>(0.0), 0.0, vec3<f32>(0.0), 0.0, KIND_LAMBERTIAN);
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // Sourced from the scene texture's own dimension, so the two always agree.
    let capacity = scene_params.scene_size.x;
    let slot = gid.x;
    if slot >= capacity {
        return;
    }

    // The ground: a sphere so large its surface reads as a plane.
    if slot == 0u {
        store_sphere(
            slot,
            vec3<f32>(0.0, -1000.0, 0.0),
            1000.0,
            vec3<f32>(0.5),
            0.0,
            KIND_LAMBERTIAN,
        );
        return;
    }
    if slot == 1u {
        store_sphere(
            slot,
            vec3<f32>(0.0, 1.0, 0.0),
            1.0,
            vec3<f32>(1.0),
            scene_params.glass_ior,
            KIND_DIELECTRIC,
        );
        return;
    }
    if slot == 2u {
        store_sphere(
            slot,
            vec3<f32>(-4.0, 1.0, 0.0),
            1.0,
            vec3<f32>(0.4, 0.2, 0.1),
            0.0,
            KIND_LAMBERTIAN,
        );
        return;
    }
    if slot == 3u {
        store_sphere(
            slot,
            vec3<f32>(4.0, 1.0, 0.0),
            1.0,
            vec3<f32>(0.7, 0.6, 0.5),
            0.0,
            KIND_METAL,
        );
        return;
    }

    // Everything past the fixed spheres is one cell of the `grid_extent` grid.
    let side = max(scene_params.grid_extent * 2u, 1u);
    let cell = slot - 4u;
    if cell >= side * side {
        store_empty(slot);
        return;
    }

    // Seeding from the cell index (not the slot) keeps a sphere's material stable
    // when the capacity changes around it.
    var rng = cell * 2654435761u + scene_params.seed;
    // Discard the first value: neighbouring seeds otherwise start correlated.
    _ = next_u32(&rng);

    let a = f32(i32(cell / side) - i32(scene_params.grid_extent));
    let b = f32(i32(cell % side) - i32(scene_params.grid_extent));
    let radius = scene_params.small_radius;
    let center = vec3<f32>(a + 0.9 * next_f32(&rng), radius, b + 0.9 * next_f32(&rng));

    // Reject any sphere that would poke into the large metal one.
    if length(center - vec3<f32>(4.0, radius, 0.0)) <= 0.9 {
        store_empty(slot);
        return;
    }

    let choice = next_f32(&rng);
    if choice < scene_params.metal_chance {
        let albedo = vec3<f32>(
            next_range(&rng, 0.5, 1.0),
            next_range(&rng, 0.5, 1.0),
            next_range(&rng, 0.5, 1.0),
        );
        store_sphere(slot, center, radius, albedo, next_range(&rng, 0.0, 0.5), KIND_METAL);
    } else if choice < scene_params.metal_chance + scene_params.glass_chance {
        store_sphere(
            slot,
            center,
            radius,
            vec3<f32>(1.0),
            scene_params.glass_ior,
            KIND_DIELECTRIC,
        );
    } else {
        // Squaring two uniform samples biases the palette toward deep colours.
        let first = vec3<f32>(next_f32(&rng), next_f32(&rng), next_f32(&rng));
        let second = vec3<f32>(next_f32(&rng), next_f32(&rng), next_f32(&rng));
        store_sphere(slot, center, radius, first * second, 0.0, KIND_LAMBERTIAN);
    }
}
