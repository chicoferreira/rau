struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
}

@vertex
fn vertex_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions = array(
        vec2f(0.0, 0.8),
        vec2f(-0.8, -0.8),
        vec2f(0.8, -0.8),
    );

    let colors = array(
        vec3f(1.0, 0.0, 0.0),
        vec3f(0.0, 1.0, 0.0),
        vec3f(0.0, 0.0, 1.0),
    );

    var output: VertexOutput;
    output.position = vec4f(positions[index], 0.0, 1.0);
    output.color = colors[index];
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4f {
    return vec4f(input.color, 1.0);
}
