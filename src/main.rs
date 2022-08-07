use log::debug;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use lumen_ray::vk_backend::BufferType;
use lumen_ray::{renderer::CPURenderer, vk_backend::VkBackend};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

// TODO: consider renaming vk_backend to vk?
// TODO: pass resolution to shaders

fn main() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(None, log::LevelFilter::Debug).init();

    let event_loop = EventLoop::new();
    let mut backend = VkBackend::new(&event_loop, "LumenRay", WIDTH, HEIGHT);

    let vs = vs::load(backend.device.clone()).unwrap();
    let fs = fs::load(backend.device.clone()).unwrap();
    backend.streaming_setup(vs.entry_point("main").unwrap(), fs.entry_point("main").unwrap());
    // TODO: let VSync be an option here

    let renderer = CPURenderer::new();

    // CPU local frame buffer
    let mut framebuffer: Vec<BufferType> = vec![0; (WIDTH * HEIGHT) as usize];

    event_loop.run(move |ev, _, control_flow| match ev {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }

        Event::RedrawEventsCleared => {
            let now = std::time::Instant::now();
            renderer.draw(&mut framebuffer, WIDTH as usize, HEIGHT as usize);
            debug!("Draw time: {:.2?}", now.elapsed());

            backend.streaming_submit(&framebuffer);
        }

        _ => (),
    });
}

#[allow(clippy::needless_question_mark)]
mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path:"shaders/cpu_render.vert"
    }
}

#[allow(clippy::needless_question_mark)]
mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path:"shaders/cpu_render.frag"
    }
}
