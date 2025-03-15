#version 450

in vec2 v_tex_coords;

out vec4 color;

void main() {
    color = vec4(v_tex_coords.x, v_tex_coords.y, 0.0, 1.0);
}