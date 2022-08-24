use std::sync::Arc;

use vulkano::{
    buffer::CpuAccessibleBuffer,
    descriptor_set::PersistentDescriptorSet,
    image::AttachmentImage,
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
};

pub type BufferType = f32;
pub const ELEM_PER_PIX: u32 = 4; // 4 f32s per pixel

pub struct StreamingBackend {
    pub pipeline: Arc<GraphicsPipeline>,
    pub viewport: Viewport,

    pub frame_staging_buffer: Arc<CpuAccessibleBuffer<[BufferType]>>,
    pub frame_image:          Arc<AttachmentImage>,
    pub set:                  Arc<PersistentDescriptorSet>,
}
