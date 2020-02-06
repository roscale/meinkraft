#version 460 core

layout (location = 0) in vec2 position;
layout (location = 1) in vec3 texture_info;

out float texture_id;
out vec2 texture_coords;

void main() {
    gl_Position = vec4(vec3(position, 0.0), 1.0);
    texture_id = texture_info.x;
    texture_coords = texture_info.yz;
}
