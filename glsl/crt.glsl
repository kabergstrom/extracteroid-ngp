// CRT post-process shared bindings.

struct CrtArgs {
    vec2 resolution;
    float time;
    float bend;
};

// @[export]
layout(set = 1, binding = 0) uniform CrtArgsBlock {
    CrtArgs args;
} crt_ubo;

// @[immutable_samplers([
//         (
//             mag_filter: Linear,
//             min_filter: Linear,
//             mip_map_mode: Linear,
//             address_mode_u: ClampToEdge,
//             address_mode_v: ClampToEdge,
//             address_mode_w: ClampToEdge,
//         )
// ])]
layout(set = 2, binding = 0) uniform sampler smp;

// @[export]
layout(set = 2, binding = 1) uniform texture2D tex;
