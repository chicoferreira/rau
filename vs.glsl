#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;

layout(set=0, binding=0)
uniform Camera {
    vec3 u_view_position;
    mat4 u_view_proj;
};

out vec3 frag_color;

void main() {
    frag_color = color;
    gl_Position = u_view_proj * vec4(position, 1.0);
}