// The "commit" half of the ping-pong.
//
// `Simulate` wrote the next generation into grid B. This pass copies it back into
// grid A so that the next step's `Simulate` reads the freshly advanced state
// again.

@group(0) @binding(0)
var grid_in: texture_2d<f32>;
@group(0) @binding(1)
var grid_out: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(grid_in);
    if gid.x >= dims.x || gid.y >= dims.y {
        return;
    }

    let coord = vec2<i32>(gid.xy);
    textureStore(grid_out, coord, textureLoad(grid_in, coord, 0));
}
