#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Mesh3dArgsStd140 {
    pub view_proj: [[f32; 4]; 4],       // +0 (size: 64)
} // 64 bytes

impl Default for Mesh3dArgsStd140 {
    fn default() -> Self {
        Mesh3dArgsStd140 {
            view_proj: <[[f32; 4]; 4]>::default(),
        }
    }
}

pub type Mesh3dArgsUniform = Mesh3dArgsStd140;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Mesh3dArgsBlockStd140 {
    pub args: Mesh3dArgsStd140,         // +0 (size: 64)
} // 64 bytes

impl Default for Mesh3dArgsBlockStd140 {
    fn default() -> Self {
        Mesh3dArgsBlockStd140 {
            args: <Mesh3dArgsStd140>::default(),
        }
    }
}

pub type Mesh3dArgsBlockUniform = Mesh3dArgsBlockStd140;

pub const TEX: crate::ShaderResourceBindingKey = crate::ShaderResourceBindingKey { set: 2, binding: 1 };
pub const INSTANCE_BUFFER: crate::ShaderResourceBindingKey = crate::ShaderResourceBindingKey { set: 0, binding: 0 };
pub const VERTEX_BUFFER: crate::ShaderResourceBindingKey = crate::ShaderResourceBindingKey { set: 1, binding: 1 };
pub const SMP: crate::ShaderResourceBindingKey = crate::ShaderResourceBindingKey { set: 2, binding: 0 };
pub const MESH3D_UBO: crate::ShaderResourceBindingKey = crate::ShaderResourceBindingKey { set: 1, binding: 0 };

