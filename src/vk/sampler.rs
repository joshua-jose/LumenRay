// describe sampler in same way as buffer or texture_array

use std::{cell::RefCell, sync::Arc};

use vulkano::{
    descriptor_set::WriteDescriptorSet,
    sampler::{Sampler as vulkanoSampler, SamplerCreateInfo},
};

use super::{HasDescriptor, VkBackend};

pub struct Sampler {
    sampler: Arc<vulkanoSampler>,
}

impl Sampler {
    pub fn new(backend: Arc<RefCell<VkBackend>>) -> Self {
        let info = SamplerCreateInfo::simple_repeat_linear_no_mipmap();
        let device = backend.borrow().device.clone();
        let sampler = vulkanoSampler::new(device, info).unwrap();

        Self { sampler }
    }
}

impl HasDescriptor for Sampler {
    fn get_descriptor(&self, binding: u32, _buffer_idx: usize) -> WriteDescriptorSet {
        WriteDescriptorSet::sampler(binding, self.sampler.clone())
    }
}
