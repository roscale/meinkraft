#version 450 core

layout (location = 0) in vec4 pos;
layout (location = 1) in vec3 texture_coords;

out VertexAttributes {
    vec3 texture_coords;
} attrs;

void main() {
    attrs.texture_coords = texture_coords;

    // Billboarding
//    mat4 model_view_matrix = view * model;
//    model_view_matrix[0][0] = 0.2;
//    model_view_matrix[1][1] = 0.2;
//    model_view_matrix[2][2] = 0.2;
//    model_view_matrix[0][1] = 0.0;
//    model_view_matrix[0][2] = 0.0;
//    model_view_matrix[1][0] = 0.0;
//    model_view_matrix[1][2] = 0.0;
//    model_view_matrix[2][0] = 0.0;
//    model_view_matrix[2][1] = 0.0;

//    gl_Position = vec4(pos.xy, 0.0, 1.0);
    gl_Position = pos;
}
