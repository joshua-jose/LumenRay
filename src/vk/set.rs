use vulkano::descriptor_set::{layout::DescriptorSetLayout, PersistentDescriptorSet};

use crate::vk::HasDescriptor;
use log::error;
use std::sync::Arc;

#[derive(Clone)]
pub struct Set {
    pub(super) descriptors: Vec<Arc<dyn HasDescriptor>>,
}

impl Set {
    pub fn new(descriptors: &[Arc<dyn HasDescriptor>]) -> Self {
        Self {
            descriptors: descriptors.to_vec(),
        }
    }

    pub fn get_descriptor_set(
        &self, layout: Arc<DescriptorSetLayout>, frame_number: usize,
    ) -> Arc<PersistentDescriptorSet> {
        let mut vk_descriptors = Vec::with_capacity(self.descriptors.len());

        let mut is_variable = false;
        let mut variable_descriptor_count = 0;

        for (binding, d) in self.descriptors.iter().enumerate() {
            vk_descriptors.push(d.get_descriptor(binding as u32, frame_number));

            // NOTE: possibly uneccesary bloat, we could just default to a count of 0
            // and do away with the boolean, it doesn't *really* matter
            if d.is_variable() {
                if is_variable {
                    error!("Variable descriptor must be the last descriptor!");
                }
                is_variable = true;
                variable_descriptor_count = d.variable_descriptor_count();
            }
        }

        //TODO: Make this a descriptor pool

        if is_variable {
            PersistentDescriptorSet::new_variable(layout, variable_descriptor_count, vk_descriptors).unwrap()
        } else {
            PersistentDescriptorSet::new(layout, vk_descriptors).unwrap()
        }
    }
}
