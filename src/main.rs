use std::sync::Arc;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::engine::{renderer::CPUStreamingRenderer, vk_backend::VkBackend};

mod engine;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(None, log::LevelFilter::Debug).init();

    let event_loop = EventLoop::new();
    let backend = Arc::new(VkBackend::new(&event_loop, "LumenRay", WIDTH, HEIGHT));
    let mut renderer = CPUStreamingRenderer::new(backend);

    let mut n = 0;

    // CPU local frame buffer
    let mut framebuffer: Vec<u32> = vec![(n % 255) + (150 << 8); (WIDTH * HEIGHT) as usize];

    event_loop.run(move |ev, _, control_flow| match ev {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }

        Event::RedrawEventsCleared => {
            n += 1;
            framebuffer.fill((n % 255) + (150 << 8));
            renderer.render(&framebuffer);
        }

        _ => (),
    });
}
