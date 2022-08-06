use std::sync::Arc;

use vulkano::{
    buffer::CpuAccessibleBuffer,
    descriptor_set::PersistentDescriptorSet,
    image::{view::ImageView, AttachmentImage, SwapchainImage},
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
    sync::GpuFuture,
};
use winit::window::Window;

pub type BufferType = u32;

pub struct StreamingPipeline {
    pub pipeline:           Arc<GraphicsPipeline>,
    pub viewport:           Viewport,
    pub attachment_views:   Vec<Arc<ImageView<SwapchainImage<Window>>>>,
    pub previous_frame_end: Option<Box<dyn GpuFuture>>,

    pub frame_staging_buffer: Arc<CpuAccessibleBuffer<[BufferType]>>,
    pub frame_image:          Arc<AttachmentImage>,
    pub set:                  Arc<PersistentDescriptorSet>,
}
