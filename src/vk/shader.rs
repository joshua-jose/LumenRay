use std::sync::Arc;

use vulkano::shader::{EntryPoint, ShaderModule};

use super::Set;

const WORKGROUP_SIZE: (u32, u32, u32) = (8, 8, 1);

pub struct Shader {
    pub(super) module:         Arc<ShaderModule>,
    pub(super) sets:           Vec<Set>,
    pub(super) workgroup_size: (u32, u32, u32),
    pub(super) dispatch_size:  DispatchSize,
}

impl Shader {
    pub fn load_from_file() {}
    pub fn load_from_module(module: Arc<ShaderModule>, sets: &[Set], dispatch_size: DispatchSize) -> Self {
        Self {
            module,
            sets: sets.to_vec(),
            workgroup_size: WORKGROUP_SIZE,
            dispatch_size,
        }
    }

    pub fn get_entry_point(&self) -> EntryPoint<'_> { self.module.entry_point("main").unwrap() }
}

pub enum DispatchSize {
    FrameResolution,
    Custom(u32, u32, u32),
}
