use log::debug;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use lumen_ray::{
    renderer::CPURenderer,
    vk_backend::{VkBackend, ELEM_PER_PIX},
};
use lumen_ray::{vk_backend::BufferType, Vec4};

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
    let mut framebuffer: Vec<Vec4> = vec![Vec4::splat(0.0); (WIDTH * HEIGHT) as usize];

    event_loop.run(move |ev, _, control_flow| match ev {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }

        Event::MainEventsCleared => {
            //let frame_start = std::time::Instant::now();
            renderer.draw(&mut framebuffer, WIDTH as usize, HEIGHT as usize);
            //debug!("Draw time: {:.2?}", frame_start.elapsed());

            // reinterpet framebuffer as a slice of f32s
            let buffer_pix = unsafe {
                std::slice::from_raw_parts_mut(
                    framebuffer.as_mut_ptr() as *mut BufferType,
                    (WIDTH * HEIGHT * ELEM_PER_PIX) as usize,
                )
            };
            //let now = std::time::Instant::now();
            backend.streaming_submit(buffer_pix);
            //debug!("Submit time: {:.2?}", now.elapsed());
            //debug!("Frame time: {:.2?}", frame_start.elapsed());
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
