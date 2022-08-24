use std::sync::Arc;

use vulkano::{image::AttachmentImage, pipeline::ComputePipeline};

pub struct ComputeBackend {
    pub pipeline:    Arc<ComputePipeline>,
    pub frame_image: Arc<AttachmentImage>,
}
