#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct NoiseArgsStd140 {
    pub resolution: [f32; 2],           // +0 (size: 8)
    pub time: f32,                      // +8 (size: 4)
    pub intensity: f32,                 // +12 (size: 4)
} // 16 bytes

impl Default for NoiseArgsStd140 {
    fn default() -> Self {
        NoiseArgsStd140 {
            resolution: <[f32; 2]>::default(),
            time: <f32>::default(),
            intensity: <f32>::default(),
        }
    }
}

pub type NoiseArgsUniform = NoiseArgsStd140;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct NoiseArgsBlockStd140 {
    pub args: NoiseArgsStd140,          // +0 (size: 16)
} // 16 bytes

impl Default for NoiseArgsBlockStd140 {
    fn default() -> Self {
        NoiseArgsBlockStd140 {
            args: <NoiseArgsStd140>::default(),
        }
    }
}

pub type NoiseArgsBlockUniform = NoiseArgsBlockStd140;

pub const NOISE_UBO: crate::ShaderResourceBindingKey = crate::ShaderResourceBindingKey { set: 1, binding: 0 };

