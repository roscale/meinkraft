#version 460 core

out vec4 fragColor;
in vec4 outColor;

void main() {
    fragColor = outColor;
}
