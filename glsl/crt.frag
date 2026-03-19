#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "crt.glsl"

layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 out_color;

// Barrel distortion for CRT curvature.
vec2 crt(vec2 coord, float bend) {
    coord = (coord - 0.5) * 2.0;
    coord *= 1.0 + pow(abs(coord.yx), vec2(2.0)) * bend;
    coord = (coord / 2.0) + 0.5;
    return coord;
}

// Sample with chromatic aberration offset.
vec3 sampleSplit(vec2 coord, float aberration) {
    vec2 offset = (coord - 0.5) * vec2(aberration);
    float r = texture(sampler2D(tex, smp), coord + offset).r;
    float g = texture(sampler2D(tex, smp), coord).g;
    float b = texture(sampler2D(tex, smp), coord - offset).b;
    return vec3(r, g, b);
}

void main() {
    float bend = crt_ubo.args.bend;
    vec2 resolution = crt_ubo.args.resolution;
    float time = crt_ubo.args.time;

    vec2 uv = crt(v_uv, bend);

    // Discard if outside CRT bounds.
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        out_color = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    // Chromatic aberration
    vec3 col = sampleSplit(uv, 0.003);

    // Scanlines
    float scanline = sin(uv.y * resolution.y * 3.14159) * 0.04;
    col -= scanline;

    // Vignette
    float vignette = uv.x * uv.y * (1.0 - uv.x) * (1.0 - uv.y);
    vignette = clamp(pow(16.0 * vignette, 0.3), 0.0, 1.0);
    col *= vignette;

    out_color = vec4(col, 1.0);
}
