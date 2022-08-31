use std::sync::Arc;

use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, BlitImageInfo, CommandBufferUsage},
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
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
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

use super::{Buffer, BufferType, ComputeContext, ComputeFrameData, HasDescriptor};

// TODO: maybe abstract away larger concepts (pipeline, swapchain, render pass) into own files/classes
#[cfg(debug_assertions)]
const ENABLE_VALIDATION_LAYERS: bool = false; //FIXME: Buggy
#[cfg(not(debug_assertions))]
const ENABLE_VALIDATION_LAYERS: bool = false;

const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_KHRONOS_validation"];

//TODO: change
const COMPUTE_WORKGROUP_X: u32 = 8;
const COMPUTE_WORKGROUP_Y: u32 = 8;

pub const FRAMES_IN_FLIGHT: usize = 1;

pub struct VkBackend {
    pub instance:              Arc<Instance>,
    pub device:                Arc<Device>,
    pub physical_device_index: usize,
    pub debug_callback:        Option<DebugUtilsMessenger>,
    pub surface:               Arc<Surface<Window>>,

    pub queues:         Vec<Arc<Queue>>,
    pub graphics_queue: Arc<Queue>,
    pub present_queue:  Arc<Queue>,
    pub compute_queue:  Arc<Queue>,

    pub swap_chain:        Option<Arc<Swapchain<Window>>>,
    pub swap_chain_images: Vec<Arc<SwapchainImage<Window>>>,
    pub attachment_views:  Vec<Arc<ImageView<SwapchainImage<Window>>>>,

    pub compute_context: Option<ComputeContext>,

    frame_number: usize,
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
            debug_callback,
            graphics_queue,
            present_queue,
            compute_queue,

            swap_chain: None,
            swap_chain_images: vec![],
            attachment_views: vec![],

            compute_context: None,
            frame_number: 0,
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
            ..DeviceExtensions::none()
        }
    }

    /// Desired features our device
    const fn get_required_device_features() -> Features {
        Features {
            runtime_descriptor_array: true,
            shader_uniform_buffer_array_non_uniform_indexing: true,
            descriptor_indexing: true,
            descriptor_binding_variable_descriptor_count: true,
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
            transfer_dst: true,
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
        // get image views to write to from swapchain
        self.attachment_views = self
            .swap_chain_images
            .iter()
            .map(|image| ImageView::new_default(image.clone()).unwrap())
            .collect::<Vec<_>>();
    }

    pub fn compute_setup(&mut self, shader: EntryPoint) {
        let pipeline = ComputePipeline::new(self.device.clone(), shader, &(), None, |layout| {
            let binding = layout[1].bindings.get_mut(&1).unwrap();
            binding.variable_descriptor_count = true;
            binding.descriptor_count = 8; //TODO: Variable
        })
        .expect("Failed to create pipeline");

        let dimensions = self.swap_chain_images[0].dimensions().width_height();

        // The Frame Data for each frame in flight
        let create_frame_data = |_| -> ComputeFrameData {
            // the framebuffer
            let frame_image = AttachmentImage::with_usage(
                self.device.clone(),
                dimensions,
                Format::R32G32B32A32_SFLOAT,
                ImageUsage {
                    storage: true,
                    transfer_src: true,
                    ..ImageUsage::none()
                },
            )
            .unwrap();

            let previous_frame_end = Some(now(self.device.clone()).boxed());

            ComputeFrameData {
                frame_image,
                previous_frame_end,
            }
        };

        let frame_data: [ComputeFrameData; FRAMES_IN_FLIGHT] = std::array::from_fn(create_frame_data);

        self.compute_context = Some(ComputeContext { pipeline, frame_data })
    }

    pub fn gen_buffer<T: BufferType>(&self, len: u64) -> Buffer<T> {
        Buffer::new(self.device.clone(), FRAMES_IN_FLIGHT + 1, len)
    }

    pub fn compute_submit<Pc>(
        &mut self, push_constants: Pc, buffers: &[&dyn HasDescriptor], textures: &[&dyn HasDescriptor],
    ) {
        let context = self.compute_context.as_mut().expect("Compute pipeline was not created");
        let swap_chain = self.swap_chain.as_ref().expect("No swapchain");
        let frame = &mut context.frame_data[self.frame_number];

        // It is important to call this function from time to time, otherwise resources will keep
        // accumulating and you will eventually reach an out of memory error.
        // Calling this function polls various fences in order to determine what the GPU has
        // already processed, and frees the resources that are no longer needed.
        frame.previous_frame_end.as_mut().unwrap().cleanup_finished();

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

        let dimensions = self.swap_chain_images[0].dimensions().width_height();

        let mut buffer_descriptors = vec![WriteDescriptorSet::image_view(
            0,
            ImageView::new_default(frame.frame_image.clone()).unwrap(),
        )];

        buffer_descriptors.reserve(buffers.len());
        let mut binding = buffer_descriptors.len() as u32;
        for b in buffers {
            buffer_descriptors.push(b.get_descriptor(binding, self.frame_number));
            binding += 1;
        }

        let mut texture_descriptors = vec![WriteDescriptorSet::sampler(
            0,
            Sampler::new(self.device.clone(), SamplerCreateInfo::simple_repeat_linear_no_mipmap()).unwrap(),
        )];
        let mut binding = texture_descriptors.len() as u32;
        for t in textures {
            texture_descriptors.push(t.get_descriptor(binding, self.frame_number));
            binding += 1;
        }

        let buf_layout = context.pipeline.layout().set_layouts().get(0).unwrap();
        let tex_layout = context.pipeline.layout().set_layouts().get(1).unwrap();

        //TODO: Make this a descriptor pool
        //TODO: allow setup of variable descriptor sets and calculate length
        let buf_set = PersistentDescriptorSet::new(buf_layout.clone(), buffer_descriptors).unwrap();
        let tex_set = PersistentDescriptorSet::new_variable(tex_layout.clone(), 7, texture_descriptors).unwrap();

        //TODO: make this multiple submit? cache command buffer
        let mut builder = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.compute_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .bind_pipeline_compute(context.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                context.pipeline.layout().clone(),
                0, // 0 is the index of our set
                (buf_set, tex_set),
            )
            .push_constants(context.pipeline.layout().clone(), 0, push_constants)
            .dispatch([
                (dimensions[0] / COMPUTE_WORKGROUP_X) + 1,
                (dimensions[1] / COMPUTE_WORKGROUP_Y) + 1,
                1,
            ])
            .unwrap()
            .blit_image(BlitImageInfo::images(
                frame.frame_image.clone(),
                self.swap_chain_images[image_num].clone(),
            ))
            .unwrap();
        let command_buffer = builder.build().unwrap();

        //TODO: Deal with swapchain recreation

        let future = frame
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.compute_queue.clone(), command_buffer)
            .unwrap()
            // This function does not actually present the image immediately. Instead it submits a
            // present command at the end of the queue. This means that it will only be presented once
            // the GPU has finished executing the command buffer that draws.
            .then_swapchain_present(self.present_queue.clone(), swap_chain.clone(), image_num)
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                frame.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                frame.previous_frame_end = Some(vulkano::sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                error!("Failed to flush future: {:?}", e);
                frame.previous_frame_end = Some(vulkano::sync::now(self.device.clone()).boxed());
            }
        }

        self.frame_number += 1;
        self.frame_number %= FRAMES_IN_FLIGHT;
    }
}
