#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct CrtArgsStd140 {
    pub resolution: [f32; 2],           // +0 (size: 8)
    pub time: f32,                      // +8 (size: 4)
    pub bend: f32,                      // +12 (size: 4)
} // 16 bytes

impl Default for CrtArgsStd140 {
    fn default() -> Self {
        CrtArgsStd140 {
            resolution: <[f32; 2]>::default(),
            time: <f32>::default(),
            bend: <f32>::default(),
        }
    }
}

pub type CrtArgsUniform = CrtArgsStd140;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct CrtArgsBlockStd140 {
    pub args: CrtArgsStd140,            // +0 (size: 16)
} // 16 bytes

impl Default for CrtArgsBlockStd140 {
    fn default() -> Self {
        CrtArgsBlockStd140 {
            args: <CrtArgsStd140>::default(),
        }
    }
}

pub type CrtArgsBlockUniform = CrtArgsBlockStd140;

pub const CRT_UBO: crate::ShaderResourceBindingKey = crate::ShaderResourceBindingKey { set: 1, binding: 0 };
pub const SMP: crate::ShaderResourceBindingKey = crate::ShaderResourceBindingKey { set: 2, binding: 0 };
pub const TEX: crate::ShaderResourceBindingKey = crate::ShaderResourceBindingKey { set: 2, binding: 1 };

