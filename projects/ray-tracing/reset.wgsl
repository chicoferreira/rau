// Clears the four accumulation buffers `trace.wgsl` writes into, which restarts
// the running average. Runs before it, and only on the frames it has to.
//
// The pass runs under the `OnChange` dispatch policy, which fires on any frame
// where one of its inputs changed. That is the whole trick behind restarting the
// average: its bind group also holds the camera, render and scene uniforms, which
// this shader never reads. A bind group forwards the data changes of everything
// it binds, so moving the camera or editing any uniform in the inspector
// re-dispatches this pass and the next trace starts from scratch. On every other
// frame nothing changed, the pass is skipped, and the image keeps converging.
//
// Bindings 4, 5 and 6 are therefore deliberately absent below.

@group(0) @binding(0)
var accumulation_red: texture_storage_2d<r32float, write>;
@group(0) @binding(1)
var accumulation_green: texture_storage_2d<r32float, write>;
@group(0) @binding(2)
var accumulation_blue: texture_storage_2d<r32float, write>;
@group(0) @binding(3)
var sample_count: texture_storage_2d<r32float, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(sample_count);
    if gid.x >= size.x || gid.y >= size.y {
        return;
    }

    let coord = vec2<i32>(gid.xy);
    textureStore(accumulation_red, coord, vec4<f32>(0.0));
    textureStore(accumulation_green, coord, vec4<f32>(0.0));
    textureStore(accumulation_blue, coord, vec4<f32>(0.0));
    textureStore(sample_count, coord, vec4<f32>(0.0));
}
