// Mesh3d shared bindings — instanced textured mesh with vertex colors.

struct Mesh3dArgs {
    mat4 view_proj;
};

// @[export]
layout(set = 1, binding = 0) uniform Mesh3dArgsBlock {
    Mesh3dArgs args;
} mesh3d_ubo;

// @[immutable_samplers([
//         (
//             mag_filter: Linear,
//             min_filter: Linear,
//             mip_map_mode: Linear,
//             address_mode_u: Repeat,
//             address_mode_v: Repeat,
//             address_mode_w: Repeat,
//         )
// ])]
layout(set = 2, binding = 0) uniform sampler smp;

// @[export]
layout(set = 2, binding = 1) uniform texture2D tex;
