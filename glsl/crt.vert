#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "crt.glsl"

layout(location = 0) out vec2 v_uv;

void main() {
    // Fullscreen triangle from gl_VertexIndex (3 vertices, no buffer needed).
    // Vertex 0: (-1, -1), Vertex 1: (3, -1), Vertex 2: (-1, 3)
    vec2 pos = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
    v_uv = pos;
    gl_Position = vec4(pos * 2.0 - 1.0, 0.0, 1.0);
}
