use std::{cell::RefCell, sync::Arc};

use vulkano::{descriptor_set::WriteDescriptorSet, image::view::ImageView};

use super::{HasDescriptor, VkBackend};

pub struct OutputImage {
    backend: Arc<RefCell<VkBackend>>,
}

impl OutputImage {
    pub fn new(backend: Arc<RefCell<VkBackend>>) -> Arc<Self> { Arc::new(Self { backend }) }
}

impl HasDescriptor for OutputImage {
    fn get_descriptor(&self, binding: u32, _buffer_idx: usize) -> WriteDescriptorSet {
        /*  We have to do an unsafe deref here, we can't borrow.
            This is because get_descriptor is generally called by compute_submit
            which requires a mutable reference to backend. This means backend will already
            be mutably borrowed when this is called, causing a runtime error.
        */
        let backend = self.backend.as_ptr();
        let view;

        unsafe {
            //TODO: try to find another way to do this?
            view = ImageView::new_default((*backend).frame_image()).unwrap();
        }
        WriteDescriptorSet::image_view(binding, view)
    }
}

// just an `Image` that then gets written to the swapchain
