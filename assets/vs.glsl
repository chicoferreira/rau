#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 tex_coords;
layout(location = 2) in vec3 normal;

layout(set=0, binding=0)
uniform Camera {
    vec3 u_view_position;
    mat4 u_view_proj;
};

layout(location = 0) out vec2 v_tex_coords;

void main() {
    v_tex_coords = tex_coords;
    gl_Position = u_view_proj * vec4(position, 1.0);
}