use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use lumen_ray::{
    renderer::{CPURenderer, SphereRenderComponent, TransformComponent},
    scene::Scene,
    vk_backend::{VkBackend, ELEM_PER_PIX},
};
use lumen_ray::{vk_backend::BufferType, Vec4};

use std::fs::File;
use std::io::prelude::*;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

// TODO: consider renaming vk_backend to vk?
// TODO: move all this to engine class

// TODO: write tests and add codecov
// TODO: add bench with default scene
// TODO: add some script that can take frame times and produce distribution/histogram

// TODO: audit game loop for unnecessary performance hits

fn main() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(None, log::LevelFilter::Debug).init();

    //let mut metric_stream = TcpStream::connect("127.0.0.1:65432").unwrap();
    let mut metric_file = File::create("metrics.csv").unwrap();

    let event_loop = EventLoop::new();
    let mut backend = VkBackend::new(&event_loop, "LumenRay", WIDTH, HEIGHT);

    let vs = vs::load(backend.device.clone()).unwrap();
    let fs = fs::load(backend.device.clone()).unwrap();
    backend.streaming_setup(vs.entry_point("main").unwrap(), fs.entry_point("main").unwrap());
    // TODO: let VSync be an option here

    let mut scene = Scene::empty();

    scene.create_entity((
        TransformComponent::with_pos(0.0, 0.0, 3.0),
        SphereRenderComponent { radius: 1.0 },
    ));
    scene.create_entity((
        TransformComponent::with_pos(-2.0, 0.0, 3.0),
        SphereRenderComponent { radius: 1.0 },
    ));
    scene.create_entity((
        TransformComponent::with_pos(2.0, 0.0, 3.0),
        SphereRenderComponent { radius: 1.0 },
    ));
    scene.create_entity((
        TransformComponent::with_pos(0.0, 2.0, 3.0),
        SphereRenderComponent { radius: 1.0 },
    ));

    scene.create_entity((
        TransformComponent::with_pos(0.0, 2.0, 8.0),
        SphereRenderComponent { radius: 5.0 },
    ));

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
            let frame_start = std::time::Instant::now();

            renderer.draw(&mut framebuffer, WIDTH as usize, HEIGHT as usize, &mut scene);
            //debug!("Draw time: {:.2?}", frame_start.elapsed());

            let draw_time = frame_start.elapsed();

            let packet = draw_time.as_nanos().to_string() + "\n";
            metric_file.write_all(packet.as_bytes()).unwrap();
            //metric_file.write_all("\n".as_bytes()).unwrap();
            // metric_stream.write_all(&packet).unwrap();

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
