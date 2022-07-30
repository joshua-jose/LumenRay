use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo,
    },
    format::Format,
    image::{view::ImageView, ImageAccess, ImageUsage, SwapchainImage},
    impl_vertex,
    instance::{
        debug::{
            DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger, DebugUtilsMessengerCreateInfo,
            Message,
        },
        Instance, InstanceCreateInfo, InstanceExtensions,
    },
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
    swapchain::{
        acquire_next_image, AcquireError, ColorSpace, PresentMode, Surface, SurfaceCapabilities, SurfaceInfo,
        Swapchain, SwapchainCreateInfo,
    },
    sync::{FlushError, GpuFuture, Sharing},
};

use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

const ENABLE_VALIDATION_LAYERS: bool = true;
const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_KHRONOS_validation"];

pub struct VkBackend {
    instance:              Instance,
    device:                Arc<Device>,
    physical_device_index: usize,
    queues:                Vec<Arc<Queue>>,
    surface:               Arc<Surface<Window>>,
    event_loop:            EventLoop<()>,
    graphics_queue:        Arc<Queue>,
    present_queue:         Arc<Queue>,
    swap_chain:            Arc<Swapchain<Window>>,
    swap_chain_images:     Vec<Arc<SwapchainImage<Window>>>,
}

impl VkBackend {
    pub fn new(title: &str, width: u32, height: u32) {
        // find out what extensions vulkano_win/winit requires
        let required_extensions = Self::get_required_instance_extensions(vulkano_win::required_extensions());
        let instance = Self::create_instance(required_extensions);
        let debug_callback = Self::setup_debug_callback(&instance);

        let (surface, event_loop) = Self::create_surface(&instance, title, width, height);

        // Choose device extensions that we're going to use.
        // In order to present images to a surface, we need a `Swapchain`, which is provided by the
        // `khr_swapchain` extension.
        let device_extensions = Self::get_required_device_extensions();

        let physical_device_index = Self::pick_physical_device(&instance, device_extensions, &surface);
        let (device, queues) = Self::create_device(&instance, physical_device_index, device_extensions);
        let (graphics_queue, present_queue, compute_queue) = Self::get_queues(queues, &surface);

        let (swap_chain, swap_chain_images) = Self::create_swap_chain(
            &instance,
            &surface,
            physical_device_index,
            &device,
            &graphics_queue,
            &present_queue,
            width,
            height,
        );

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
            CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices).unwrap();

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

        let vs = vs::load(device.clone()).unwrap();
        let fs = fs::load(device.clone()).unwrap();

        let pipeline = GraphicsPipeline::start()
            // We describe the formats of attachment images where the colors, depth and/or stencil
            // information will be written. The pipeline will only be usable with this particular
            // configuration of the attachment images.
            .render_pass(PipelineRenderingCreateInfo {
                // We specify a single color attachment that will be rendered to. When we begin
                // rendering, we will specify a swapchain image to be used as this attachment, so here
                // we set its format to be the same format as the swapchain.
                color_attachment_formats: vec![Some(swap_chain.image_format())],
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
            .build(device.clone())
            .unwrap();

        let dimensions = swap_chain_images[0].dimensions().width_height();

        let mut viewport = Viewport {
            origin:      [0.0, 0.0],
            dimensions:  [dimensions[0] as f32, dimensions[1] as f32],
            depth_range: 0.0..1.0,
        };

        let mut attachment_views = swap_chain_images
            .iter()
            .map(|image| ImageView::new_default(image.clone()).unwrap())
            .collect::<Vec<_>>();

        let mut previous_frame_end = Some(vulkano::sync::now(device.clone()).boxed());

        event_loop.run(move |ev, _, control_flow| match ev {
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
                let (image_num, suboptimal, acquire_future) = match acquire_next_image(swap_chain.clone(), None) {
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
                    device.clone(),
                    graphics_queue.family(),
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
                            clear_value: Some([0.0, 0.0, 1.0, 1.0].into()),
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
                    .then_execute(graphics_queue.clone(), command_buffer)
                    .unwrap()
                    // The color output is now expected to contain our triangle. But in order to show it on
                    // the screen, we have to *present* the image by calling `present`.
                    //
                    // This function does not actually present the image immediately. Instead it submits a
                    // present command at the end of the queue. This means that it will only be presented once
                    // the GPU has finished executing the command buffer that draws the triangle.
                    .then_swapchain_present(present_queue.clone(), swap_chain.clone(), image_num)
                    .then_signal_fence_and_flush();

                match future {
                    Ok(future) => {
                        previous_frame_end = Some(future.boxed());
                    }
                    Err(FlushError::OutOfDate) => {
                        previous_frame_end = Some(vulkano::sync::now(device.clone()).boxed());
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(vulkano::sync::now(device.clone()).boxed());
                    }
                }
            }

            _ => (),
        });
    }

    /*
    ----------------------------------------------------------------------------------------------------------------------
                                        VULKAN CONFIGURATION AND OPTIONS
    ----------------------------------------------------------------------------------------------------------------------
    */

    const fn get_required_instance_extensions(window_extensions: InstanceExtensions) -> InstanceExtensions {
        InstanceExtensions {
            ext_debug_utils: true,
            ..InstanceExtensions::none()
        }
        .union(&window_extensions)
    }

    const fn get_required_device_extensions() -> DeviceExtensions {
        DeviceExtensions {
            khr_swapchain: true,
            khr_dynamic_rendering: true,
            ..DeviceExtensions::none()
        }
    }

    const fn get_required_device_features() -> Features {
        Features {
            dynamic_rendering: true,
            ..Features::none()
        }
    }

    fn is_device_suitable<W>(p: &PhysicalDevice, device_extensions: DeviceExtensions, surface: &Surface<W>) -> bool {
        p.supported_extensions().is_superset_of(&device_extensions)
        &&
        // look for the right queue families 
        // The device may have multiple queue families, that can only perform one task (graphics, present, transfer, compute)
        // we look for at least one family that can do these
        p.queue_families().find(|&q| q.supports_graphics()).is_some() &&
        p.queue_families().find(|&q| q.supports_compute()).is_some() &&
        p.queue_families().find(|&q| q.supports_surface(&surface).unwrap_or(false)).is_some()
    }

    fn choose_swap_surface_format(available_formats: Vec<(Format, ColorSpace)>) -> (Format, ColorSpace) {
        // Try to use our preferred format and color space (8 bit RGB in the sRGB colour space)
        *available_formats
            .iter()
            .find(|(format, color_space)| *format == Format::B8G8R8A8_SRGB && *color_space == ColorSpace::SrgbNonLinear)
            .expect("Desired colour format and space not available")
    }

    fn choose_swap_present_mode(mut available_present_modes: Vec<PresentMode>) -> PresentMode {
        // score present modes based on how desirable they are, with lowest being best
        available_present_modes.sort_by_key(|m| match m {
            PresentMode::Mailbox => 0,
            PresentMode::Immediate => 1,
            PresentMode::Fifo => 2,
            _ => 100,
        });
        *available_present_modes.first().expect("No present modes")
    }

    fn choose_swap_extent(capabilities: &SurfaceCapabilities, width: u32, height: u32) -> [u32; 2] {
        // try to determine the dimensions of the swapchain.
        // we would like this to be our window width and height.
        if let Some(current_extent) = capabilities.current_extent {
            return current_extent;
        } else {
            let mut actual_extent = [width, height];
            actual_extent[0] =
                capabilities.min_image_extent[0].max(capabilities.max_image_extent[0].min(actual_extent[0]));
            actual_extent[1] =
                capabilities.min_image_extent[1].max(capabilities.max_image_extent[1].min(actual_extent[1]));
            actual_extent
        }
    }

    /*
    ----------------------------------------------------------------------------------------------------------------------
    */

    fn get_queues(queues: Vec<Arc<Queue>>, surface: &Surface<Window>) -> (Arc<Queue>, Arc<Queue>, Arc<Queue>) {
        let graphics_queue = queues
            .iter()
            .find(|q| q.family().supports_graphics())
            .expect("Cannot find graphics queue");
        let present_queue = queues
            .iter()
            .find(|q| q.family().supports_surface(surface).unwrap_or(false))
            .expect("Cannot find present queue");
        let compute_queue = queues
            .iter()
            .find(|q| q.family().supports_compute())
            .expect("Cannot find compute queue");

        (graphics_queue.clone(), present_queue.clone(), compute_queue.clone())
    }

    fn create_instance(extensions: InstanceExtensions) -> Arc<Instance> {
        let mut enabled_layers: Vec<String> = vec![];
        // push on validation layers if required
        if ENABLE_VALIDATION_LAYERS {
            enabled_layers.extend(VALIDATION_LAYERS.iter().map(|s| s.to_string()));
        }
        // create a new Vulkan instance
        Instance::new(InstanceCreateInfo {
            enabled_extensions: extensions,
            // Enable enumerating devices that use non-conformant vulkan implementations. (ex. MoltenVK)
            enumerate_portability: true,
            enabled_layers,
            ..Default::default()
        })
        .expect("failed to create instance")
    }

    fn setup_debug_callback(instance: &Arc<Instance>) -> Option<DebugUtilsMessenger> {
        if !ENABLE_VALIDATION_LAYERS {
            return None;
        }

        let log_message = Arc::new(|msg: &Message| println!("validation layer: {:?}", msg.description));

        unsafe {
            Some(
                DebugUtilsMessenger::new(
                    instance.clone(),
                    DebugUtilsMessengerCreateInfo {
                        message_severity: DebugUtilsMessageSeverity::errors_and_warnings(),
                        message_type: DebugUtilsMessageType::all(),
                        ..DebugUtilsMessengerCreateInfo::user_callback(log_message)
                    },
                )
                .expect("Could not create debug messenger"),
            )
        }
    }

    fn create_surface(
        instance: &Arc<Instance>, title: &str, width: u32, height: u32,
    ) -> (Arc<Surface<Window>>, EventLoop<()>) {
        //first we need to create the window.
        //
        // This is done by creating a `WindowBuilder` from the `winit` crate, then calling the
        // `build_vk_surface` method provided by the `VkSurfaceBuild` trait from `vulkano_win`. If you
        // ever get an error about `build_vk_surface` being undefined in one of your projects, this
        // probably means that you forgot to import this trait.
        //
        // This returns a `vulkano::swapchain::Surface` object that contains both a cross-platform winit
        // window and a cross-platform Vulkan surface that represents the surface of the window.
        let event_loop = EventLoop::new();
        let surface = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(width, height))
            .with_resizable(false)
            .build_vk_surface(&event_loop, instance.clone())
            .expect("Couldn't build surface");
        (surface, event_loop)
    }

    fn pick_physical_device<W>(
        instance: &Arc<Instance>, device_extensions: DeviceExtensions, surface: &Surface<W>,
    ) -> usize {
        let mut sorted_devices = PhysicalDevice::enumerate(&instance).enumerate().collect::<Vec<_>>();
        sorted_devices.sort_by_key(|(_, p)| {
            // We assign a lower score to device types that are likely to be faster/better.
            match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
            }
        });

        // find the first device deemed "suitable"
        let (physical_device_index, physical_device) = sorted_devices
            .iter()
            .find(|(_, device)| Self::is_device_suitable(&device, device_extensions, surface))
            .expect("failed to find a suitable GPU!");

        // debug info
        println!(
            "Using device: {} (type: {:?}, vk version: {})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
            physical_device.properties().api_version,
        );
        *physical_device_index
    }

    fn create_device(
        instance: &Arc<Instance>, physical_device_index: usize, device_extensions: DeviceExtensions,
    ) -> (Arc<Device>, Vec<Arc<Queue>>) {
        // Now initializing the device. This is probably the most important object of Vulkan.
        let physical_device = PhysicalDevice::from_index(&instance, physical_device_index).unwrap();

        // get a list of every queue family available
        let queue_create_infos = physical_device
            .queue_families()
            .map(|family| QueueCreateInfo::family(family))
            .collect();

        // NOTE: the tutorial recommends passing the validation layers as well
        // for legacy reasons (if ENABLE_VALIDATION_LAYERS is true). Vulkano handles that
        // for us internally.

        // create logical device
        let (device, queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                enabled_features: Self::get_required_device_features(),
                queue_create_infos,
                ..Default::default()
            },
        )
        .expect("failed to create logical device!");

        (device, queues.collect())
    }

    fn create_swap_chain(
        instance: &Arc<Instance>, surface: &Arc<Surface<Window>>, physical_device_index: usize, device: &Arc<Device>,
        graphics_queue: &Arc<Queue>, present_queue: &Arc<Queue>, width: u32, height: u32,
    ) -> (Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>) {
        let physical_device = PhysicalDevice::from_index(&instance, physical_device_index).unwrap();

        let capabilities = physical_device
            .surface_capabilities(surface, SurfaceInfo::default())
            .expect("failed to get surface capabilities");

        let available_formats = physical_device
            .surface_formats(surface, SurfaceInfo::default())
            .expect("Cannot get surface formats");

        let available_present_modes = physical_device
            .surface_present_modes(surface)
            .expect("Cannot get surface present modes")
            .collect();

        // determine desired swapchain properties
        let surface_format = Self::choose_swap_surface_format(available_formats);
        let present_mode = Self::choose_swap_present_mode(available_present_modes);
        let extent = Self::choose_swap_extent(&capabilities, width, height);

        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count.is_some() && image_count > capabilities.max_image_count.unwrap() {
            image_count = capabilities.max_image_count.unwrap();
        }

        let image_usage = ImageUsage {
            color_attachment: true,
            ..ImageUsage::none()
        };
        let sharing = if graphics_queue == present_queue {
            Sharing::Exclusive
        } else {
            Sharing::Concurrent(vec![graphics_queue.family().id(), present_queue.family().id()].into())
        };

        let (swap_chain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: image_count,
                image_format: Some(surface_format.0),
                image_color_space: surface_format.1,
                image_usage,
                image_extent: extent,
                image_array_layers: 1,
                image_sharing: sharing,
                present_mode,
                clipped: true,
                ..Default::default()
            },
        )
        .expect("failed to create swap chain!");

        (swap_chain, images)
    }
}
