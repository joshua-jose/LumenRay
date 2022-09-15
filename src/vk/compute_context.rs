use std::sync::Arc;

use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer},
    image::{AttachmentImage, ImageAccess},
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
    sync::GpuFuture,
};

use super::{DispatchSize, Shader, VkBackend, FRAMES_IN_FLIGHT};

pub struct ComputeFrameData {
    pub frame_image:        Arc<AttachmentImage>,
    pub previous_frame_end: Option<Box<dyn GpuFuture>>,
}

pub struct ComputeContext {
    pub pipelines:  Vec<Arc<ComputePipeline>>,
    pub shaders:    Vec<Shader>,
    pub frame_data: [ComputeFrameData; FRAMES_IN_FLIGHT],
}

pub struct ComputeSubmitBuilder<'a> {
    backend:                     &'a mut VkBackend,
    pub(super) command_builders: Vec<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
}

impl<'a> ComputeSubmitBuilder<'a> {
    pub fn new(backend: &'a mut VkBackend) -> Self {
        Self {
            backend,
            command_builders: vec![],
        }
    }

    pub fn add_shader_execution<Pc>(
        &mut self, shader_idx: usize, dispatch_size: DispatchSize, push_constants: Option<Pc>,
    ) -> &mut Self {
        let context = self
            .backend
            .compute_context
            .as_ref()
            .expect("Compute pipeline was not created");

        let shader = &context.shaders[shader_idx];
        let pipeline = &context.pipelines[shader_idx];

        let dimensions = self.backend.swap_chain_images[0].dimensions().width_height();
        let layouts = pipeline.layout().set_layouts();
        let mut vk_sets = Vec::with_capacity(shader.sets.len());

        for (set_number, set) in shader.sets.iter().enumerate() {
            let layout = layouts.get(set_number).unwrap();
            let vk_set = set.get_descriptor_set(layout.clone(), self.backend.frame_number);
            vk_sets.push(vk_set);
        }

        //TODO: make this multiple submit? cache command buffer
        let mut builder = AutoCommandBufferBuilder::primary(
            self.backend.device.clone(),
            self.backend.compute_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder.bind_pipeline_compute(pipeline.clone()).bind_descriptor_sets(
            PipelineBindPoint::Compute,
            pipeline.layout().clone(),
            0, // 0 is the index of the first set
            vk_sets,
        );
        // check if this shader takes push constants
        if let Some(pc) = push_constants {
            builder.push_constants(pipeline.layout().clone(), 0, pc);
        }

        let group_counts = match dispatch_size {
            DispatchSize::FrameResolution => [
                (dimensions[0] / shader.workgroup_size.0) + 1,
                (dimensions[1] / shader.workgroup_size.1) + 1,
                1,
            ],
            DispatchSize::Custom(x, y, z) => [
                (x / shader.workgroup_size.0) + 1,
                (y / shader.workgroup_size.1) + 1,
                (z / shader.workgroup_size.2) + 1,
            ],
        };

        builder.dispatch(group_counts).unwrap();
        self.command_builders.push(builder);

        self
    }
    pub fn submit(self) {
        //TODO: we could just have the user pass this to backend submit, rather than having a child function to do it

        let ptr: *mut VkBackend = self.backend;
        unsafe {
            (*ptr).compute_submit(self);
        }
    }
}
