#version 450 core

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

layout (location = 0) in vec3 pos;
layout (location = 1) in vec2 texture_coords;
//
out VertexAttributes {
    vec3 frag_pos;
    vec2 texture_coords;
} attrs;

void main() {
    attrs.frag_pos = vec3(model * vec4(pos, 1.0f));
    attrs.texture_coords = texture_coords;
    gl_Position = projection * view * vec4(attrs.frag_pos, 1.0f);
    attrs.frag_pos = vec3(view * model * vec4(pos, 1.0f));
}