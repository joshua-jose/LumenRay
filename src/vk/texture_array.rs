use std::{
    cell::RefCell,
    sync::{Arc, RwLock},
};

use log::debug;
use vulkano::{
    descriptor_set::WriteDescriptorSet,
    device::Queue,
    image::{view::ImageView, ImageViewAbstract, ImmutableImage, MipmapsCount},
    sync::GpuFuture,
};

use super::{HasDescriptor, VkBackend};

pub struct TextureArray {
    queue:    Arc<Queue>,
    textures: RwLock<Vec<Arc<ImmutableImage>>>,
}

impl TextureArray {
    pub fn new(backend: Arc<RefCell<VkBackend>>) -> Self {
        Self {
            queue:    backend.borrow().transfer_queue.clone(),
            textures: RwLock::new(vec![]),
        }
    }

    pub fn push_texture(&self, width: u32, height: u32, data: Vec<f32>) {
        let (image, future) = ImmutableImage::from_iter(
            data,
            vulkano::image::ImageDimensions::Dim2d {
                width,
                height,
                array_layers: 1,
            },
            MipmapsCount::One,
            vulkano::format::Format::R32G32B32A32_SFLOAT,
            self.queue.clone(),
        )
        .unwrap();
        debug!("Uploading a texture to GPU");
        future.flush().unwrap();
        self.textures.write().unwrap().push(image);
    }

    //pub fn set_texture(&mut self, id: u32, ...)
}

impl HasDescriptor for TextureArray {
    fn get_descriptor(&self, binding: u32, _frame_number: usize) -> WriteDescriptorSet {
        let textures_reader = self.textures.read().unwrap();
        let views = textures_reader
            .iter()
            .map(|image| ImageView::new_default(image.clone()).unwrap() as Arc<dyn ImageViewAbstract>);

        WriteDescriptorSet::image_view_array(binding, 0, views)
    }

    fn is_variable(&self) -> bool { true }

    fn variable_descriptor_count(&self) -> u32 { self.textures.read().unwrap().len() as u32 }
}
