#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "../../newgameplus2/newgameplus/glsl/vertex_formats.glsl"
#include "mesh3d.glsl"

layout(location = 0) in vec2 in_uv;

#if defined(HAS_COLOR)
layout(location = 1) in vec4 in_color;
#endif

layout(location = 0) out vec4 out_color;

void main() {
    vec4 base = texture(sampler2D(tex, smp), in_uv);

#if defined(HAS_COLOR)
    base *= in_color;
#endif

    out_color = base;
}
