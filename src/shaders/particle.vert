#version 450 core

layout (location = 0) in vec4 pos;
layout (location = 1) in vec3 texture_coords;

out VertexAttributes {
    vec3 texture_coords;
} attrs;

void main() {
    attrs.texture_coords = texture_coords;
    gl_Position = pos;
}
