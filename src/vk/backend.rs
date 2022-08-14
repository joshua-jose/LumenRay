use std::sync::Arc;

use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, RenderingAttachmentInfo, RenderingInfo,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo,
    },
    format::Format,
    image::{view::ImageView, AttachmentImage, ImageAccess, ImageUsage, SwapchainImage},
    instance::{
        debug::{
            DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger, DebugUtilsMessengerCreateInfo,
            Message,
        },
        Instance, InstanceCreateInfo, InstanceExtensions,
    },
    pipeline::{
        graphics::{
            render_pass::PipelineRenderingCreateInfo,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{LoadOp, StoreOp},
    sampler::{Sampler, SamplerCreateInfo},
    shader::EntryPoint,
    swapchain::{
        acquire_next_image, AcquireError, ColorSpace, PresentMode, Surface, SurfaceCapabilities, SurfaceInfo,
        Swapchain, SwapchainCreateInfo,
    },
    sync::{now, FlushError, GpuFuture, Sharing},
};

use log::{debug, error, info, trace, warn};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use super::{BufferType, StreamingPipeline, ELEM_PER_PIX};

// TODO: maybe abstract away larger concepts (pipeline, swapchain, render pass) into own files/classes

const ENABLE_VALIDATION_LAYERS: bool = true;
const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_KHRONOS_validation"];

pub struct VkBackend {
    pub instance:              Arc<Instance>,
    pub device:                Arc<Device>,
    pub physical_device_index: usize,
    pub debug_callback:        Option<DebugUtilsMessenger>,
    pub queues:                Vec<Arc<Queue>>,
    pub surface:               Arc<Surface<Window>>,
    pub graphics_queue:        Arc<Queue>,
    pub present_queue:         Arc<Queue>,
    pub compute_queue:         Arc<Queue>,
    pub swap_chain:            Option<Arc<Swapchain<Window>>>,
    pub swap_chain_images:     Vec<Arc<SwapchainImage<Window>>>,

    pub streaming_pipeline: Option<StreamingPipeline>,
}

impl VkBackend {
    pub fn new(event_loop: &EventLoop<()>, title: &str, width: u32, height: u32) -> Self {
        // find out what extensions vulkano_win/winit requires
        let required_extensions = Self::get_required_instance_extensions(vulkano_win::required_extensions());
        let instance = Self::create_instance(required_extensions);
        let debug_callback = Self::setup_debug_callback(&instance);

        let surface = Self::create_surface(&instance, event_loop, title, width, height);

        let device_extensions = Self::get_required_device_extensions();

        let physical_device_index = Self::pick_physical_device(&instance, device_extensions, &surface);
        let (device, queues) = Self::create_device(&instance, physical_device_index, device_extensions);
        let (graphics_queue, present_queue, compute_queue) = Self::get_queues(&queues, &surface);

        let mut this = Self {
            instance,
            device,
            physical_device_index,
            queues,
            surface,
            graphics_queue,
            present_queue,
            swap_chain: None,
            swap_chain_images: vec![],
            debug_callback,
            compute_queue,
            streaming_pipeline: None,
        };
        this.create_swap_chain(width, height);
        this
    }

    // ----------------------------------------------------------------------------------------------------------------------
    //                                      VULKAN CONFIGURATION AND OPTIONS
    // ----------------------------------------------------------------------------------------------------------------------

    /// Desired extensions for a given instance
    const fn get_required_instance_extensions(window_extensions: InstanceExtensions) -> InstanceExtensions {
        InstanceExtensions {
            ext_debug_utils: true,
            ..InstanceExtensions::none()
        }
        .union(&window_extensions)
    }

    /// Desired extensions for our device
    const fn get_required_device_extensions() -> DeviceExtensions {
        DeviceExtensions {
            khr_swapchain: true,
            khr_dynamic_rendering: true,
            ..DeviceExtensions::none()
        }
    }

    /// Desired features our device
    const fn get_required_device_features() -> Features {
        Features {
            dynamic_rendering: true,
            ..Features::none()
        }
    }

    /// Decides if a given physical device has the right extensions and queues for us
    fn is_device_suitable<W>(p: &PhysicalDevice, device_extensions: DeviceExtensions, surface: &Surface<W>) -> bool {
        p.supported_extensions().is_superset_of(&device_extensions)
        &&
        // look for the right queue families 
        // The device may have multiple queue families, that can only perform one task (graphics, present, transfer, compute)
        // we look for at least one family that can do these
        p.queue_families().any(|q| q.supports_graphics()) &&
        p.queue_families().any(|q| q.supports_compute()) &&
        p.queue_families().any(|q| q.supports_surface(surface).unwrap_or(false))
    }

    /// Picks a colour format,and a colour space to use.
    fn choose_swap_surface_format(available_formats: Vec<(Format, ColorSpace)>) -> (Format, ColorSpace) {
        // Try to use our preferred format and color space (8 bit RGB in the sRGB colour space)
        debug!("Available formats: {:?}", available_formats);
        *available_formats
            .iter()
            .find(|(format, color_space)| *format == Format::B8G8R8A8_SRGB && *color_space == ColorSpace::SrgbNonLinear)
            .expect("Desired colour format and space not available")
    }

    /// Picks a present mode, based on a score. The lowest scoring present mode is selected
    fn choose_swap_present_mode(mut available_present_modes: Vec<PresentMode>) -> PresentMode {
        // score present modes based on how desirable they are, with lowest being best
        available_present_modes.sort_by_key(|m| match m {
            PresentMode::Mailbox => 1,
            PresentMode::Immediate => 2,
            PresentMode::Fifo => 0,
            _ => 100,
        });
        *available_present_modes.first().expect("No present modes")
    }

    /// The size of the swap chain images. We would like this to be our surface width and height
    fn choose_swap_extent(capabilities: &SurfaceCapabilities, width: u32, height: u32) -> [u32; 2] {
        // try to determine the dimensions of the swapchain.
        // we would like this to be our window width and height.
        if let Some(current_extent) = capabilities.current_extent {
            current_extent
        } else {
            let mut actual_extent = [width, height];
            actual_extent[0] =
                capabilities.min_image_extent[0].max(capabilities.max_image_extent[0].min(actual_extent[0]));
            actual_extent[1] =
                capabilities.min_image_extent[1].max(capabilities.max_image_extent[1].min(actual_extent[1]));
            actual_extent
        }
    }

    // ----------------------------------------------------------------------------------------------------------------------
    //                                      VULKAN SETUP AND INITIALISATION
    // ----------------------------------------------------------------------------------------------------------------------

    /// Gets the queues that we want from a list of queues, that was provided by the device.
    fn get_queues(queues: &[Arc<Queue>], surface: &Surface<Window>) -> (Arc<Queue>, Arc<Queue>, Arc<Queue>) {
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

    /// Creates a Vulkan instance
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

    /// Creates a debug callback, if the validation layer is enabled. This allows the validation layer to give us debug messages.
    fn setup_debug_callback(instance: &Arc<Instance>) -> Option<DebugUtilsMessenger> {
        if !ENABLE_VALIDATION_LAYERS {
            return None;
        }

        // the logging callback
        let log_message = Arc::new(|msg: &Message| {
            let msg_type = if msg.ty.general {
                "validation_layer/general"
            } else if msg.ty.validation {
                "validation_layer/validation"
            } else if msg.ty.performance {
                "validation_layer/performance"
            } else {
                "validation_layer/unknown"
            };
            if msg.severity.error {
                error!(target: msg_type, "{}", msg.description);
            } else if msg.severity.warning {
                warn!(target: msg_type, "{}", msg.description);
            } else if msg.severity.information {
                trace!(target: msg_type, "{}", msg.description);
            } else if msg.severity.verbose {
                trace!(target: msg_type, "{}", msg.description);
            }
        });

        // setup/register the callback
        unsafe {
            Some(
                DebugUtilsMessenger::new(
                    instance.clone(),
                    DebugUtilsMessengerCreateInfo {
                        message_severity: DebugUtilsMessageSeverity::all(),
                        message_type: DebugUtilsMessageType::all(),
                        ..DebugUtilsMessengerCreateInfo::user_callback(log_message)
                    },
                )
                .expect("Could not create debug messenger"),
            )
        }
    }

    /// Creates a window and a vulkan surface.
    fn create_surface(
        instance: &Arc<Instance>, event_loop: &EventLoop<()>, title: &str, width: u32, height: u32,
    ) -> Arc<Surface<Window>> {
        //first we need to create the window.
        //
        // This is done by creating a `WindowBuilder` from the `winit` crate, then calling the
        // `build_vk_surface` method provided by the `VkSurfaceBuild` trait from `vulkano_win`. If you
        // ever get an error about `build_vk_surface` being undefined in one of your projects, this
        // probably means that you forgot to import this trait.
        //
        // This returns a `vulkano::swapchain::Surface` object that contains both a cross-platform winit
        // window and a cross-platform Vulkan surface that represents the surface of the window.
        WindowBuilder::new()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(width, height))
            .with_resizable(false)
            .build_vk_surface(event_loop, instance.clone())
            .expect("Couldn't build surface")
    }

    /// Picks out a physical device that has the lowest score (best performing device), and is deemed "suitable"
    fn pick_physical_device<W>(
        instance: &Arc<Instance>, device_extensions: DeviceExtensions, surface: &Surface<W>,
    ) -> usize {
        let mut sorted_devices = PhysicalDevice::enumerate(instance).enumerate().collect::<Vec<_>>();
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

        if !sorted_devices[0]
            .1
            .supported_extensions()
            .is_superset_of(&device_extensions)
        {
            let missing = device_extensions.difference(sorted_devices[0].1.supported_extensions());
            warn!(
                "Ideal device is missing extensions: {:?}\nTry updating your graphics drivers",
                missing
            );
        }

        // find the first device deemed "suitable"
        let (physical_device_index, physical_device) = sorted_devices
            .iter()
            .find(|(_, device)| Self::is_device_suitable(device, device_extensions, surface))
            .expect("failed to find a suitable GPU!");

        // debug info
        info!(
            "Using device: {} (type: {:?}, vk version: {})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
            physical_device.properties().api_version,
        );

        *physical_device_index
    }

    /// From a physical device, create a logical device, that is our method of talking to the physical device
    fn create_device(
        instance: &Arc<Instance>, physical_device_index: usize, device_extensions: DeviceExtensions,
    ) -> (Arc<Device>, Vec<Arc<Queue>>) {
        // Now initializing the device. This is probably the most important object of Vulkan.
        let physical_device = PhysicalDevice::from_index(instance, physical_device_index).unwrap();

        // get a list of every queue family available
        let queue_create_infos = physical_device.queue_families().map(QueueCreateInfo::family).collect();

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

    /// Creates a swap chain, which we will render to
    pub fn create_swap_chain(&mut self, width: u32, height: u32) {
        let physical_device = PhysicalDevice::from_index(&self.instance, self.physical_device_index).unwrap();

        // Find out the capabilities of the device given this surface
        let capabilities = physical_device
            .surface_capabilities(&self.surface, SurfaceInfo::default())
            .expect("failed to get surface capabilities");

        let available_formats = physical_device
            .surface_formats(&self.surface, SurfaceInfo::default())
            .expect("Cannot get surface formats");

        let available_present_modes = physical_device
            .surface_present_modes(&self.surface)
            .expect("Cannot get surface present modes")
            .collect();

        // based on thosse capabilities, pick out formats and modes
        let surface_format = Self::choose_swap_surface_format(available_formats);
        let present_mode = Self::choose_swap_present_mode(available_present_modes);
        let extent = Self::choose_swap_extent(&capabilities, width, height);

        // number of swap chain images
        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count.is_some() && image_count > capabilities.max_image_count.unwrap() {
            image_count = capabilities.max_image_count.unwrap();
        }

        // what this swapchain image is going to be used for
        // This is for colour attachment to a framebuffer
        let image_usage = ImageUsage {
            color_attachment: true,
            ..ImageUsage::none()
        };
        // how to share swapchain resources.
        let sharing = if self.graphics_queue == self.present_queue {
            Sharing::Exclusive
        } else {
            Sharing::Concurrent(vec![self.graphics_queue.family().id(), self.present_queue.family().id()].into())
        };

        let (swap_chain, images) = Swapchain::new(
            self.device.clone(),
            self.surface.clone(),
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

        self.swap_chain = Some(swap_chain);
        self.swap_chain_images = images;
    }

    pub fn streaming_setup(&mut self, vert_s: EntryPoint, frag_s: EntryPoint) {
        let swap_chain = self.swap_chain.as_ref().expect("No swapchain");

        // dimensions of our viewport
        let dimensions = self.swap_chain_images[0].dimensions().width_height();
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
                color_attachment_formats: vec![Some(swap_chain.image_format())],
                ..Default::default()
            })
            // A Vulkan shader can in theory contain multiple entry points, so we have to specify
            // which one.
            .vertex_shader(vert_s, ())
            // Use a resizable viewport set to draw over the entire window
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport.clone()]))
            // See `vertex_shader`.
            .fragment_shader(frag_s, ())
            // Now that our builder is filled, we call `build()` to obtain an actual pipeline.
            .build(self.device.clone())
            .unwrap();

        // get image views to write to from swapchain
        let attachment_views = self
            .swap_chain_images
            .iter()
            .map(|image| ImageView::new_default(image.clone()).unwrap())
            .collect::<Vec<_>>();

        // help with syncing
        let previous_frame_end = Some(now(self.device.clone()).boxed());

        // We write to this buffer from the CPU side, where each frame will be uploaded to the GPU
        let frame_staging_buffer = unsafe {
            CpuAccessibleBuffer::uninitialized_array(
                self.device.clone(),
                (dimensions[0] * dimensions[1] * ELEM_PER_PIX) as u64,
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
            self.device.clone(),
            dimensions,
            Format::R32G32B32A32_SFLOAT,
            ImageUsage {
                transfer_dst: true,
                sampled: true,
                ..ImageUsage::none()
            },
        )
        .unwrap();

        // setup the image we will write to from the CPU
        let layout = pipeline.layout().set_layouts().get(0).unwrap();
        let sampler = Sampler::new(self.device.clone(), SamplerCreateInfo::simple_repeat_linear()).unwrap();
        let image_view = ImageView::new_default(frame_image.clone()).unwrap();
        let set = PersistentDescriptorSet::new(
            layout.clone(),
            [WriteDescriptorSet::image_view_sampler(0, image_view, sampler)],
        )
        .unwrap();

        self.streaming_pipeline = Some(StreamingPipeline {
            pipeline,
            viewport,
            attachment_views,
            previous_frame_end,
            frame_staging_buffer,
            frame_image,
            set,
        });
    }

    pub fn streaming_submit(&mut self, framebuffer: &[BufferType]) {
        //! This function sends a framebuffer off to the GPU
        //! It starts by writing to the staging buffer, then acquiring the swapchain image to write to
        //! It then creates a command buffer, which will copy from the transfer buffer
        //! to the GPU side framebuffer image,then sets up a render pass to blit that to the
        //! swapchain, after going through a few shaders.

        // It is important to call this function from time to time, otherwise resources will keep
        // accumulating and you will eventually reach an out of memory error.
        // Calling this function polls various fences in order to determine what the GPU has
        // already processed, and frees the resources that are no longer needed.
        let mut pipeline = self
            .streaming_pipeline
            .as_mut()
            .expect("Streaming pipeline was not created");

        pipeline.previous_frame_end.as_mut().unwrap().cleanup_finished();

        // get the swapchain
        let swap_chain = self.swap_chain.as_ref().expect("No swapchain");

        {
            match pipeline.frame_staging_buffer.write() {
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
        let (image_num, _suboptimal, acquire_future) = match acquire_next_image(swap_chain.clone(), None) {
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
        //TODO: make this multiple submit? cache command buffer
        let mut builder = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.graphics_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                pipeline.frame_staging_buffer.clone(),
                pipeline.frame_image.clone(),
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
                        pipeline.attachment_views[image_num].clone(),
                    )
                })],
                ..Default::default()
            })
            .unwrap()
            // We are now inside the first subpass of the render pass. We add a draw command.
            //
            // The last two parameters contain the list of resources to pass to the shaders.
            // Since we used an `EmptyPipeline` object, the objects have to be `()`.
            .set_viewport(0, [pipeline.viewport.clone()])
            .bind_pipeline_graphics(pipeline.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.pipeline.layout().clone(),
                0,
                pipeline.set.clone(),
            )
            .draw(6, 1, 0, 0)
            .unwrap()
            // We leave the render pass.
            .end_rendering()
            .unwrap();

        // Finish building the command buffer by calling `build`.
        let command_buffer = builder.build().unwrap();

        let future = pipeline
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.graphics_queue.clone(), command_buffer)
            .unwrap()
            // The color output is now expected to contain our triangle. But in order to show it on
            // the screen, we have to *present* the image by calling `present`.
            //
            // This function does not actually present the image immediately. Instead it submits a
            // present command at the end of the queue. This means that it will only be presented once
            // the GPU has finished executing the command buffer that draws the triangle.
            .then_swapchain_present(self.present_queue.clone(), swap_chain.clone(), image_num)
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                pipeline.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                pipeline.previous_frame_end = Some(vulkano::sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                error!("Failed to flush future: {:?}", e);
                pipeline.previous_frame_end = Some(vulkano::sync::now(self.device.clone()).boxed());
            }
        }
    }
}