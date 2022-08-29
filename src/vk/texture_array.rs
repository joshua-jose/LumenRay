use std::{cell::RefCell, sync::Arc};

use log::debug;
use vulkano::{
    descriptor_set::WriteDescriptorSet,
    device::{Device, Queue},
    image::{view::ImageView, ImageViewAbstract, ImmutableImage, MipmapsCount},
    sampler::{Sampler, SamplerCreateInfo},
    sync::GpuFuture,
};

use super::{HasDescriptor, VkBackend};

pub struct TextureArray {
    device:   Arc<Device>,
    queue:    Arc<Queue>,
    textures: Vec<Arc<ImmutableImage>>,
}

impl TextureArray {
    pub fn new(backend: Arc<RefCell<VkBackend>>) -> Self {
        Self {
            device:   backend.borrow().device.clone(),
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
        let mut views = self
            .textures
            .iter()
            .map(|image| ImageView::new_default(image.clone()).unwrap() as Arc<dyn ImageViewAbstract>);

        //let sampler = Sampler::new(self.device.clone(), SamplerCreateInfo::simple_repeat_linear_no_mipmap()).unwrap();

        //WriteDescriptorSet::image_view_sampler(binding, views.next().unwrap(), sampler)

        WriteDescriptorSet::image_view_array(binding, 0, views)
    }
}
