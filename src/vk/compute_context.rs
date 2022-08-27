use std::sync::Arc;

use vulkano::{image::AttachmentImage, pipeline::ComputePipeline, sync::GpuFuture};

use super::FRAMES_IN_FLIGHT;

pub struct ComputeFrameData {
    pub frame_image:        Arc<AttachmentImage>,
    pub previous_frame_end: Option<Box<dyn GpuFuture>>,
}

pub struct ComputeContext {
    pub pipeline: Arc<ComputePipeline>,

    pub frame_data: [ComputeFrameData; FRAMES_IN_FLIGHT],
}
