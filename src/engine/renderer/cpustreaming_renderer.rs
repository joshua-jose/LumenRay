use std::sync::Arc;

use log::{error, trace};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, RenderingAttachmentInfo, RenderingInfo,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    image::{attachment::AttachmentImage, view::ImageView, ImageAccess, ImageUsage, SwapchainImage},
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            render_pass::PipelineRenderingCreateInfo,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{LoadOp, StoreOp},
    sampler::{Sampler, SamplerCreateInfo},
    swapchain::{acquire_next_image, AcquireError},
    sync::{FlushError, GpuFuture},
};
use winit::window::Window;

use crate::engine::vk_backend::VkBackend;

pub struct CPUStreamingRenderer {
    backend:            Arc<VkBackend>,
    pipeline:           Arc<GraphicsPipeline>,
    viewport:           Viewport,
    attachment_views:   Vec<Arc<ImageView<SwapchainImage<Window>>>>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,

    frame_staging_buffer: Arc<CpuAccessibleBuffer<[u32]>>,
    frame_image:          Arc<AttachmentImage>,
    set:                  Arc<PersistentDescriptorSet>,
}

#[allow(clippy::needless_question_mark)]
mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450
            out gl_PerVertex {
                vec4 gl_Position;
            };
            
            layout(location = 0) out vec3 fragColor;
            
            vec2 positions[6] = vec2[](
                vec2(-1.0, -1.0),
                vec2(-1.0, 1.0),
                vec2(1.0, -1.0),
                
                vec2(1.0, 1.0),
                vec2(-1.0, 1.0),
                vec2(1.0, -1.0)
            );
            
            vec3 colors[6] = vec3[](
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
                vec3(0.0, 0.0, 1.0),
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
                vec3(0.0, 0.0, 1.0)
            );
            void main() {
                gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
                fragColor = colors[gl_VertexIndex];
            }
        "
    }
}

#[allow(clippy::needless_question_mark)]
mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            layout(location = 0) in vec3 fragColor;
            layout(location = 0) out vec4 f_color;

            layout(set = 0, binding = 0) uniform sampler2D tex;

            in vec4 gl_FragCoord;

            vec2 iResolution = vec2(800,600);

            float rand(vec2 co){
                return fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
            }

            void main() {
                vec2 uv = gl_FragCoord.xy / iResolution.xy;
                uv.y = 1.0-uv.y;  // flip
                f_color = texture(tex, uv);
            }
        "
    }
}

impl CPUStreamingRenderer {
    pub fn new(backend: Arc<VkBackend>) -> Self {
        // vertex and fragment shaders
        let vs = vs::load(backend.device.clone()).unwrap();
        let fs = fs::load(backend.device.clone()).unwrap();

        // dimensions of our viewport
        let dimensions = backend.swap_chain_images[0].dimensions().width_height();
        let viewport = Viewport {
            origin:      [0.0, 0.0],
            dimensions:  [dimensions[0] as f32, dimensions[1] as f32],
            depth_range: 0.0..1.0,
        };

        // Set up our graphics pipeline
        let pipeline = GraphicsPipeline::start()
            .render_pass(PipelineRenderingCreateInfo {
                // We specify a single color attachment that will be rendered to. When we begin
                // rendering, we will specify a swapchain image to be used as this attachment, so here
                // we set its format to be the same format as the swapchain.
                color_attachment_formats: vec![Some(backend.swap_chain.image_format())],
                ..Default::default()
            })
            // The content of the vertex buffer describes a list of triangles.
            .input_assembly_state(InputAssemblyState::new())
            // A Vulkan shader can in theory contain multiple entry points, so we have to specify
            // which one.
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            // Use a resizable viewport set to draw over the entire window
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport.clone()]))
            // See `vertex_shader`.
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            // Now that our builder is filled, we call `build()` to obtain an actual pipeline.
            .build(backend.device.clone())
            .unwrap();

        let attachment_views = backend
            .swap_chain_images
            .iter()
            .map(|image| ImageView::new_default(image.clone()).unwrap())
            .collect::<Vec<_>>();

        let previous_frame_end = Some(vulkano::sync::now(backend.device.clone()).boxed());

        // We write to this buffer from the CPU side, where each frame will be uploaded to the GPU
        let frame_staging_buffer = unsafe {
            CpuAccessibleBuffer::uninitialized_array(
                backend.device.clone(),
                (dimensions[0] * dimensions[1]) as u64,
                BufferUsage {
                    transfer_src: true,
                    ..BufferUsage::none()
                },
                false,
            )
            .unwrap()
        };

        // the destination image that will be sampled
        let frame_image = AttachmentImage::with_usage(
            backend.device.clone(),
            dimensions,
            backend.swap_chain.image_format(),
            ImageUsage {
                transfer_dst: true,
                sampled: true,
                ..ImageUsage::none()
            },
        )
        .unwrap();

        let layout = pipeline.layout().set_layouts().get(0).unwrap();
        let sampler = Sampler::new(backend.device.clone(), SamplerCreateInfo::simple_repeat_linear()).unwrap();
        let image_view = ImageView::new_default(frame_image.clone()).unwrap();
        let set = PersistentDescriptorSet::new(
            layout.clone(),
            [WriteDescriptorSet::image_view_sampler(0, image_view, sampler)],
        )
        .unwrap();

        Self {
            backend,
            pipeline,
            viewport,
            attachment_views,
            previous_frame_end,
            frame_staging_buffer,
            frame_image,
            set,
        }
    }

    pub fn render(&mut self, framebuffer: &[u32]) {
        // It is important to call this function from time to time, otherwise resources will keep
        // accumulating and you will eventually reach an out of memory error.
        // Calling this function polls various fences in order to determine what the GPU has
        // already processed, and frees the resources that are no longer needed.
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        {
            match self.frame_staging_buffer.write() {
                Ok(mut writer) => writer.copy_from_slice(framebuffer),
                Err(e) => {
                    // if the frame rate is super high, we could be trying to write to this buffer *while* the previous frame is still copying
                    // from the buffer to the image! In this case just log it and skip over
                    trace!("Frame staging buffer write error: {}", e);
                }
            }
        }

        // Before we can draw on the output, we have to *acquire* an image from the swapchain. If
        // no image is available (which happens if you submit draw commands too quickly), then the
        // function will block.
        // This operation returns the index of the image that we are allowed to draw upon.
        //
        // This function can block if no image is available. The parameter is an optional timeout
        // after which the function call will return an error.
        let (image_num, _suboptimal, acquire_future) = match acquire_next_image(self.backend.swap_chain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                return;
            }
            Err(e) => panic!("Failed to acquire next image: {:?}", e),
        };

        // In order to draw, we have to build a *command buffer*. The command buffer object holds
        // the list of commands that are going to be executed.
        //
        // Building a command buffer is an expensive operation (usually a few hundred
        // microseconds), but it is known to be a hot path in the driver and is expected to be
        // optimized.
        //
        // Note that we have to pass a queue family when we create the command buffer. The command
        // buffer will only be executable on that given queue family.
        let mut builder = AutoCommandBufferBuilder::primary(
            self.backend.device.clone(),
            self.backend.graphics_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                self.frame_staging_buffer.clone(),
                self.frame_image.clone(),
            ))
            .unwrap();

        builder
            // Before we can draw, we have to *enter a render pass*. We specify which
            // attachments we are going to use for rendering here, which needs to match
            // what was previously specified when creating the pipeline.
            .begin_rendering(RenderingInfo {
                // As before, we specify one color attachment, but now we specify
                // the image view to use as well as how it should be used.
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    // `Clear` means that we ask the GPU to clear the content of this
                    // attachment at the start of rendering.
                    load_op: LoadOp::Clear,
                    // `Store` means that we ask the GPU to store the rendered output
                    // in the attachment image. We could also ask it to discard the result.
                    store_op: StoreOp::Store,
                    // The value to clear the attachment with. Here we clear it with a
                    // blue color.
                    //
                    // Only attachments that have `LoadOp::Clear` are provided with
                    // clear values, any others should use `None` as the clear value.
                    clear_value: Some([0.0, 0.0, 0.0, 1.0].into()),
                    ..RenderingAttachmentInfo::image_view(
                        // We specify image view corresponding to the currently acquired
                        // swapchain image, to use for this attachment.
                        self.attachment_views[image_num].clone(),
                    )
                })],
                ..Default::default()
            })
            .unwrap()
            // We are now inside the first subpass of the render pass. We add a draw command.
            //
            // The last two parameters contain the list of resources to pass to the shaders.
            // Since we used an `EmptyPipeline` object, the objects have to be `()`.
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                self.set.clone(),
            )
            .draw(6, 1, 0, 0)
            .unwrap()
            // We leave the render pass.
            .end_rendering()
            .unwrap();

        // Finish building the command buffer by calling `build`.
        let command_buffer = builder.build().unwrap();

        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.backend.graphics_queue.clone(), command_buffer)
            .unwrap()
            // The color output is now expected to contain our triangle. But in order to show it on
            // the screen, we have to *present* the image by calling `present`.
            //
            // This function does not actually present the image immediately. Instead it submits a
            // present command at the end of the queue. This means that it will only be presented once
            // the GPU has finished executing the command buffer that draws the triangle.
            .then_swapchain_present(
                self.backend.present_queue.clone(),
                self.backend.swap_chain.clone(),
                image_num,
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.previous_frame_end = Some(vulkano::sync::now(self.backend.device.clone()).boxed());
            }
            Err(e) => {
                error!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(vulkano::sync::now(self.backend.device.clone()).boxed());
            }
        }
    }
}
