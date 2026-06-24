// One generation of Conway's Game of Life.
//
// Reads the current state from `grid_in` (bound as a sampled texture) and writes
// the next generation into `grid_out` (a write-only storage texture). Reading and
// writing two *different* textures is what makes this race-free: every neighbour
// lookup happens on the stable input while results land in the output buffer.
//
// This pass re-runs on a fixed cadence via its `Periodic` dispatch policy; the
// shader itself has no notion of time.

@group(0) @binding(0)
var grid_in: texture_2d<f32>;
@group(0) @binding(1)
var grid_out: texture_storage_2d<rgba8unorm, write>;

// Samples a cell with toroidal (wrap-around) coordinates so the world has no edges.
fn cell(coord: vec2<i32>, dims: vec2<i32>) -> u32 {
    let wrapped = (coord + dims) % dims;
    return u32(textureLoad(grid_in, wrapped, 0).r > 0.5);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(grid_in);
    if gid.x >= size.x || gid.y >= size.y {
        return;
    }

    let dims = vec2<i32>(size);
    let pos = vec2<i32>(gid.xy);

    var neighbours: u32 = 0u;
    for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            if dx == 0 && dy == 0 {
                continue;
            }
            neighbours = neighbours + cell(pos + vec2<i32>(dx, dy), dims);
        }
    }

    let alive = cell(pos, dims) == 1u;

    // Conway's rules: a live cell survives with 2 or 3 neighbours; a dead cell
    // is born with exactly 3.
    var next: u32;
    if alive {
        next = u32(neighbours == 2u || neighbours == 3u);
    } else {
        next = u32(neighbours == 3u);
    }

    let value = f32(next);
    textureStore(grid_out, pos, vec4<f32>(vec3<f32>(value), 1.0));
}
