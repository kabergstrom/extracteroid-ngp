#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "../../newgameplus2/newgameplus/glsl/vertex_formats.glsl"
#include "mesh3d.glsl"

// @[vertex_formats(["PU", "PCU", "PNU", "PNUC"])]
layout(set = 1, binding = 1) buffer readonly VertexBuffer {
    Vertex vertices[];
} vertex_buffer;

layout(set = 0, binding = 0) buffer readonly InstanceData {
    mat4 models[];
} instance_buffer;

layout(location = 0) out vec2 out_uv;

#if defined(HAS_COLOR)
layout(location = 1) out vec4 out_color;
#endif

void main() {
    Vertex v = vertex_buffer.vertices[gl_VertexIndex];
    mat4 model = instance_buffer.models[gl_InstanceIndex];

    gl_Position = mesh3d_ubo.args.view_proj * model * vec4(v.pos, 1.0);

#if defined(HAS_UV0)
    out_uv = v.uv;
#else
    out_uv = vec2(0.0);
#endif

#if defined(HAS_COLOR)
    uint c = v.color;
    out_color = vec4(
        float(c & 0xFFu) / 255.0,
        float((c >> 8u) & 0xFFu) / 255.0,
        float((c >> 16u) & 0xFFu) / 255.0,
        float((c >> 24u) & 0xFFu) / 255.0
    );
#endif
}
