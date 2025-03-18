// Based on https://www.shadertoy.com/view/lXXXzS

#version 450

layout(set = 1, binding = 0) uniform TimeUniform {
    float time;
    float pad1;
    float pad2;
    float pad3;
};

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 fragColor;

void main() {
    vec2 p = tex_coords * 6.0;

    for (float i = 0.0; i < 8.0; i++) {
        p.x += sin(p.y + i + time * 0.3);
        p *= mat2(6, -8, 8, 6) / 8.0;
    }
    fragColor = sin(p.xyxy * 0.3 + vec4(0, 1, 2, 3)) * 0.5 + 0.5;
}