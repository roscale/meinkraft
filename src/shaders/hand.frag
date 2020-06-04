#version 450 core

out vec4 Color;

uniform sampler2DArray tex;

in VertexAttributes {
    vec3 texture_coords;
    vec3 normal;
} attrs;

void main() {
    vec4 diffuse_frag = texture(tex, attrs.texture_coords);
    if (diffuse_frag.a == 0.0) {
        discard;
    }
    Color = diffuse_frag;
//    Color.rgb *= attrs.normal.x;
    Color.rgb *= 1.0 - abs(attrs.normal.z) * 0.2;
    Color.rgb *= 1.0 - abs(attrs.normal.x) * 0.4;

//    if (attrs.normal.z == 1.0) {
//        Color.rgb *= 0.5;
//    } else if (attrs.normal.x == -1.0) {
//        Color.rgb *= 0.7;
//    }
}
