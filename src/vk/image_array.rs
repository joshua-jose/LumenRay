//TODO: could combine with texture_array, they are similar, could be done with generics

use std::{
    cell::RefCell,
    sync::{Arc, RwLock},
};

use vulkano::{
    descriptor_set::WriteDescriptorSet,
    device::Device,
    image::{view::ImageView, AttachmentImage, ImageUsage, ImageViewAbstract},
};

use super::{HasDescriptor, VkBackend};

pub struct ImageArray {
    device: Arc<Device>,
    images: RwLock<Vec<Arc<AttachmentImage>>>,
}

impl ImageArray {
    pub fn new(backend: Arc<RefCell<VkBackend>>) -> Self {
        Self {
            device: backend.borrow().device.clone(),
            images: RwLock::new(vec![]),
        }
    }

    pub fn push_image(&self, width: u32, height: u32) {
        let image = AttachmentImage::with_usage(
            self.device.clone(),
            [width, height],
            vulkano::format::Format::R32G32B32A32_SFLOAT,
            ImageUsage {
                sampled: true,
                storage: true,
                ..ImageUsage::none()
            },
        )
        .unwrap();
        self.images.write().unwrap().push(image);
    }

    pub fn push_images(&self, width: u32, height: u32, count: usize) {
        for _ in 0..count {
            self.push_image(width, height);
        }
    }

    //pub fn set_texture(&mut self, id: u32, ...)
}

impl HasDescriptor for ImageArray {
    fn get_descriptor(&self, binding: u32, _frame_number: usize) -> WriteDescriptorSet {
        let textures_reader = self.images.read().unwrap();
        let views = textures_reader
            .iter()
            .map(|image| ImageView::new_default(image.clone()).unwrap() as Arc<dyn ImageViewAbstract>);

        WriteDescriptorSet::image_view_array(binding, 0, views)
    }

    fn is_variable(&self) -> bool { true }

    fn variable_descriptor_count(&self) -> u32 { self.images.read().unwrap().len() as u32 }
}
