use vulkano::descriptor_set::WriteDescriptorSet;

pub trait HasDescriptor {
    fn get_descriptor(&self, binding: u32, buffer_idx: usize) -> WriteDescriptorSet;

    fn is_variable(&self) -> bool { false }
    fn variable_descriptor_count(&self) -> u32 { 0 }
    fn max_size(&self) -> u32 { 1024 }
}
