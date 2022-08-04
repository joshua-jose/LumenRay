#![feature(core_intrinsics)]
use std::sync::Arc;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::engine::vec3;
use crate::engine::{ray_engine::CPURayEngine, renderer::CPUStreamingRenderer, vk_backend::VkBackend};

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
    let mut ray_engine = CPURayEngine::new();

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
            let rays = ray_engine.cast_sight_rays(WIDTH as usize, HEIGHT as usize);
            n += 1;
            //let colour_wave_1 = 255.0 / 2.0 * (1.0 + (n as f32 / 20.0).sin());
            //let colour_wave_2 = 255.0 / 2.0 * (1.0 + (n as f32 / 20.0).cos());

            //let colour_wave_1 = colour_wave_1.trunc() as u32;
            //let colour_wave_2 = colour_wave_2.trunc() as u32;

            for (i, h) in rays.iter().enumerate() {
                if h.is_some() {
                    let info = h.as_ref().unwrap();
                    let normal = info.normal;
                    let col = normal.dot((vec3(0.0, 4.0, -1.0) - info.position).normalize()).max(0.0) * 255.0;

                    framebuffer[i] = col.trunc() as u32;
                    // framebuffer[i] = ((1.0 + normal.x) * 255.0 / 2.0).trunc() as u32
                    //     + ((((1.0 + normal.y) * 255.0 / 2.0).trunc() as u32) << 8)
                    //     + ((((1.0 + normal.z) * 255.0 / 2.0).trunc() as u32) << 16);

                    //framebuffer[i] = 255;
                    //colour_wave_1 + (100 << 16);
                } else {
                    framebuffer[i] = 0; //(colour_wave_2 << 8) + (150 << 16);
                }
            }
            //framebuffer.fill(colour_wave_1.trunc() as u32 + ((colour_wave_2.trunc() as u32) << 8) + (150 << 16));
            renderer.render(&framebuffer);
        }

        _ => (),
    });
}
