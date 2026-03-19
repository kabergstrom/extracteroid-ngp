// Static noise post-process shared bindings.

struct NoiseArgs {
    vec2 resolution;
    float time;
    float intensity;
};

// @[export]
layout(set = 1, binding = 0) uniform NoiseArgsBlock {
    NoiseArgs args;
} noise_ubo;
