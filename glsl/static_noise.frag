#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "static_noise.glsl"

layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 out_color;

// Hash-based pseudo-random noise.
float rand(vec2 co) {
    return fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
}

void main() {
    float time = noise_ubo.args.time;
    float intensity = noise_ubo.args.intensity;
    vec2 resolution = noise_ubo.args.resolution;

    vec2 pixel = floor(v_uv * resolution);
    float noise = rand(pixel + vec2(time * 100.0));

    out_color = vec4(vec3(noise), intensity);
}
