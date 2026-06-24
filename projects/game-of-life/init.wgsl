// Seeds the simulation with a pseudo-random "soup" of live cells.
//
// This compute pass uses the `OnChange` dispatch policy, so it only runs once
// when the project is first built. The `Simulate` and `Copy` passes that follow
// are the ones that re-dispatch on a timer.

@group(0) @binding(0)
var grid_out: texture_storage_2d<rgba8unorm, write>;
// Seed for the hash below; change it to get a different starting soup.
@group(0) @binding(1)
var<uniform> seed: u32;

// Cheap integer hash (a few xorshift/multiply rounds)
// used to scatter live cells across the grid.
fn hash(p: vec2<u32>) -> u32 {
    var x = p.x * 1973u + p.y * 9277u + seed;
    x = (x ^ (x >> 13u)) * 12741277u;
    x = x ^ (x >> 16u);
    return x;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(grid_out);
    if gid.x >= dims.x || gid.y >= dims.y {
        return;
    }

    // About 30% of the cells start alive, which gives a lively soup.
    let alive = f32(hash(gid.xy) % 100u < 30u);

    textureStore(grid_out, vec2<i32>(gid.xy), vec4<f32>(vec3<f32>(alive), 1.0));
}
