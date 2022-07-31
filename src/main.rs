use bytemuck::{Pod, Zeroable};
use log::error;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo},
    image::{view::ImageView, ImageAccess},
    impl_vertex,
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            render_pass::PipelineRenderingCreateInfo,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline,
    },
    render_pass::{LoadOp, StoreOp},
    swapchain::{acquire_next_image, AcquireError},
    sync::{FlushError, GpuFuture},
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

mod engine;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(None, log::LevelFilter::Trace).init();
    println!("Hello World!");
    let backend = engine::vk_backend::VkBackend::new("LumenRay", 800, 600);

    // Triangle

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
    struct Vertex {
        position: [f32; 2],
    }
    impl_vertex!(Vertex, position);

    let vertices = [
        Vertex {
            position: [-0.5, -0.25],
        },
        Vertex { position: [0.0, 0.5] },
        Vertex { position: [0.25, -0.1] },
    ];
    let vertex_buffer =
        CpuAccessibleBuffer::from_iter(backend.device.clone(), BufferUsage::all(), false, vertices).unwrap();

    mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: "
                        #version 450
                        out gl_PerVertex {
                            vec4 gl_Position;
                        };
                        
                        layout(location = 0) out vec3 fragColor;
                        
                        vec2 positions[3] = vec2[](
                            vec2(0.0, -0.5),
                            vec2(0.5, 0.5),
                            vec2(-0.5, 0.5)
                        );
                        
                        vec3 colors[3] = vec3[](
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

    mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: "
                        #version 450
                        layout(location = 0) in vec3 fragColor;
                        layout(location = 0) out vec4 f_color;
                        void main() {
                            f_color = vec4(fragColor, 1.0);
                        }
                    "
        }
    }

    let vs = vs::load(backend.device.clone()).unwrap();
    let fs = fs::load(backend.device.clone()).unwrap();

    let pipeline = GraphicsPipeline::start()
        // We describe the formats of attachment images where the colors, depth and/or stencil
        // information will be written. The pipeline will only be usable with this particular
        // configuration of the attachment images.
        .render_pass(PipelineRenderingCreateInfo {
            // We specify a single color attachment that will be rendered to. When we begin
            // rendering, we will specify a swapchain image to be used as this attachment, so here
            // we set its format to be the same format as the swapchain.
            color_attachment_formats: vec![Some(backend.swap_chain.image_format())],
            ..Default::default()
        })
        // We need to indicate the layout of the vertices.
        .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
        // The content of the vertex buffer describes a list of triangles.
        .input_assembly_state(InputAssemblyState::new())
        // A Vulkan shader can in theory contain multiple entry points, so we have to specify
        // which one.
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        // Use a resizable viewport set to draw over the entire window
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        // See `vertex_shader`.
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        // Now that our builder is filled, we call `build()` to obtain an actual pipeline.
        .build(backend.device.clone())
        .unwrap();

    let dimensions = backend.swap_chain_images[0].dimensions().width_height();

    let viewport = Viewport {
        origin:      [0.0, 0.0],
        dimensions:  [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };

    let attachment_views = backend
        .swap_chain_images
        .iter()
        .map(|image| ImageView::new_default(image.clone()).unwrap())
        .collect::<Vec<_>>();

    let mut previous_frame_end = Some(vulkano::sync::now(backend.device.clone()).boxed());

    backend.event_loop.run(move |ev, _, control_flow| match ev {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }

        Event::RedrawEventsCleared => {
            // It is important to call this function from time to time, otherwise resources will keep
            // accumulating and you will eventually reach an out of memory error.
            // Calling this function polls various fences in order to determine what the GPU has
            // already processed, and frees the resources that are no longer needed.
            previous_frame_end.as_mut().unwrap().cleanup_finished();

            // Before we can draw on the output, we have to *acquire* an image from the swapchain. If
            // no image is available (which happens if you submit draw commands too quickly), then the
            // function will block.
            // This operation returns the index of the image that we are allowed to draw upon.
            //
            // This function can block if no image is available. The parameter is an optional timeout
            // after which the function call will return an error.
            let (image_num, _suboptimal, acquire_future) = match acquire_next_image(backend.swap_chain.clone(), None) {
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
                backend.device.clone(),
                backend.graphics_queue.family(),
                CommandBufferUsage::OneTimeSubmit,
            )
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
                            attachment_views[image_num].clone(),
                        )
                    })],
                    ..Default::default()
                })
                .unwrap()
                // We are now inside the first subpass of the render pass. We add a draw command.
                //
                // The last two parameters contain the list of resources to pass to the shaders.
                // Since we used an `EmptyPipeline` object, the objects have to be `()`.
                .set_viewport(0, [viewport.clone()])
                .bind_pipeline_graphics(pipeline.clone())
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                // We leave the render pass.
                .end_rendering()
                .unwrap();

            // Finish building the command buffer by calling `build`.
            let command_buffer = builder.build().unwrap();

            let future = previous_frame_end
                .take()
                .unwrap()
                .join(acquire_future)
                .then_execute(backend.graphics_queue.clone(), command_buffer)
                .unwrap()
                // The color output is now expected to contain our triangle. But in order to show it on
                // the screen, we have to *present* the image by calling `present`.
                //
                // This function does not actually present the image immediately. Instead it submits a
                // present command at the end of the queue. This means that it will only be presented once
                // the GPU has finished executing the command buffer that draws the triangle.
                .then_swapchain_present(backend.present_queue.clone(), backend.swap_chain.clone(), image_num)
                .then_signal_fence_and_flush();

            match future {
                Ok(future) => {
                    previous_frame_end = Some(future.boxed());
                }
                Err(FlushError::OutOfDate) => {
                    previous_frame_end = Some(vulkano::sync::now(backend.device.clone()).boxed());
                }
                Err(e) => {
                    error!("Failed to flush future: {:?}", e);
                    previous_frame_end = Some(vulkano::sync::now(backend.device.clone()).boxed());
                }
            }
        }

        _ => (),
    });
}
