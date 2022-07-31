use std::sync::Arc;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::engine::{renderer::CPUStreamingRenderer, vk_backend::VkBackend};

mod engine;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(None, log::LevelFilter::Debug).init();
    println!("Hello World!");
    let event_loop = EventLoop::new();
    let backend = Arc::new(VkBackend::new(&event_loop, "LumenRay", 800, 600));
    let mut renderer = CPUStreamingRenderer::new(backend);

    event_loop.run(move |ev, _, control_flow| match ev {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }

        Event::RedrawEventsCleared => {
            renderer.render();
        }

        _ => (),
    });
}
