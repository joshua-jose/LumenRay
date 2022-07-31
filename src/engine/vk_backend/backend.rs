use std::sync::Arc;

use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo,
    },
    format::Format,
    image::{ImageUsage, SwapchainImage},
    instance::{
        debug::{
            DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger, DebugUtilsMessengerCreateInfo,
            Message,
        },
        Instance, InstanceCreateInfo, InstanceExtensions,
    },
    swapchain::{ColorSpace, PresentMode, Surface, SurfaceCapabilities, SurfaceInfo, Swapchain, SwapchainCreateInfo},
    sync::Sharing,
};

use log::{debug, error, info, warn};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

const ENABLE_VALIDATION_LAYERS: bool = true;
const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_KHRONOS_validation"];

pub struct VkBackend {
    pub instance:              Arc<Instance>,
    pub device:                Arc<Device>,
    pub physical_device_index: usize,
    pub debug_callback:        Option<DebugUtilsMessenger>,
    pub queues:                Vec<Arc<Queue>>,
    pub surface:               Arc<Surface<Window>>,
    pub event_loop:            EventLoop<()>,
    pub graphics_queue:        Arc<Queue>,
    pub present_queue:         Arc<Queue>,
    pub compute_queue:         Arc<Queue>,
    pub swap_chain:            Arc<Swapchain<Window>>,
    pub swap_chain_images:     Vec<Arc<SwapchainImage<Window>>>,
}

impl VkBackend {
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        // find out what extensions vulkano_win/winit requires
        let required_extensions = Self::get_required_instance_extensions(vulkano_win::required_extensions());
        let instance = Self::create_instance(required_extensions);
        let debug_callback = Self::setup_debug_callback(&instance);

        let (surface, event_loop) = Self::create_surface(&instance, title, width, height);

        let device_extensions = Self::get_required_device_extensions();

        let physical_device_index = Self::pick_physical_device(&instance, device_extensions, &surface);
        let (device, queues) = Self::create_device(&instance, physical_device_index, device_extensions);
        let (graphics_queue, present_queue, compute_queue) = Self::get_queues(&queues, &surface);

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

        Self {
            instance,
            device,
            physical_device_index,
            queues,
            surface,
            event_loop,
            graphics_queue,
            present_queue,
            swap_chain,
            swap_chain_images,
            debug_callback,
            compute_queue,
        }
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

    fn get_queues(queues: &Vec<Arc<Queue>>, surface: &Surface<Window>) -> (Arc<Queue>, Arc<Queue>, Arc<Queue>) {
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

        let log_message = Arc::new(|msg: &Message| {
            let msg_type = if msg.ty.general {
                "general"
            } else if msg.ty.validation {
                "validation"
            } else if msg.ty.performance {
                "performance"
            } else {
                " unknown"
            };
            if msg.severity.error {
                error!("{}: {}", msg_type, msg.description);
            } else if msg.severity.warning {
                warn!("{}: {}", msg_type, msg.description);
            } else if msg.severity.information {
                info!("{}: {}", msg_type, msg.description);
            } else if msg.severity.verbose {
                debug!("{}: {}", msg_type, msg.description);
            }
        });

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

        if !sorted_devices[0]
            .1
            .supported_extensions()
            .is_superset_of(&device_extensions)
        {
            let missing = device_extensions.difference(&sorted_devices[0].1.supported_extensions());
            warn!(
                "Ideal device is missing extensions: {:?}\nTry updating your graphics drivers",
                missing
            );
        }

        // find the first device deemed "suitable"
        let (physical_device_index, physical_device) = sorted_devices
            .iter()
            .find(|(_, device)| Self::is_device_suitable(&device, device_extensions, surface))
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
