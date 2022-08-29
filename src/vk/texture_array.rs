use std::{cell::RefCell, sync::Arc};

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
    textures: Vec<Arc<ImmutableImage>>,
}

impl TextureArray {
    pub fn new(backend: Arc<RefCell<VkBackend>>) -> Self {
        Self {
            queue:    backend.borrow().compute_queue.clone(),
            textures: vec![],
        }
    }

    pub fn push_texture(&mut self, width: u32, height: u32, data: Vec<f32>) {
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
        self.textures.push(image);
    }

    //pub fn set_texture(&mut self, id: u32, ...)
}

impl HasDescriptor for TextureArray {
    fn get_descriptor(&self, binding: u32, _frame_number: usize) -> WriteDescriptorSet {
        let views = self
            .textures
            .iter()
            .map(|image| ImageView::new_default(image.clone()).unwrap() as Arc<dyn ImageViewAbstract>);

        WriteDescriptorSet::image_view_array(binding, 0, views)
    }
}
