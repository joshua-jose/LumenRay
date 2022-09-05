use std::sync::Arc;

use vulkano::shader::{EntryPoint, ShaderModule};

use super::Set;

pub struct Shader {
    pub(super) module: Arc<ShaderModule>,
    pub(super) sets:   Vec<Set>,
}

impl Shader {
    pub fn load_from_file() {}
    pub fn load_from_module(module: Arc<ShaderModule>, sets: &[Set]) -> Self {
        Self {
            module,
            sets: sets.to_vec(),
        }
    }

    pub fn get_entry_point(&self) -> EntryPoint<'_> { self.module.entry_point("main").unwrap() }
}
