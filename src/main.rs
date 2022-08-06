#![feature(core_intrinsics)]

use engine::vk_backend::BufferType;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::engine::{renderer::CPURenderer, vk_backend::VkBackend};

mod engine;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(None, log::LevelFilter::Debug).init();

    let event_loop = EventLoop::new();
    let mut backend = VkBackend::new(&event_loop, "LumenRay", WIDTH, HEIGHT);

    let vs = vs::load(backend.device.clone()).unwrap();
    let fs = fs::load(backend.device.clone()).unwrap();
    backend.streaming_setup(vs.entry_point("main").unwrap(), fs.entry_point("main").unwrap());
    // TODO: let VSync be an option here
    /*
    TODO: merge renderer and ray engine, since they already perform tightly knit jobs, and
    will effectively be one eventually
    */
    // TODO: move contents of engine/ into src/, its redundant
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
            // let now = std::time::Instant::now();
            // println!("Frame time: {:.2?}", now.elapsed());

            renderer.draw(&mut framebuffer, WIDTH as usize, HEIGHT as usize);
            backend.streaming_submit(&framebuffer);

            //
        }

        _ => (),
    });
}

#[allow(clippy::needless_question_mark)]
mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450
            out gl_PerVertex {
                vec4 gl_Position;
            };
            
            layout(location = 0) out vec3 fragColor;
            
            vec2 positions[6] = vec2[](
                vec2(-1.0, -1.0),
                vec2(-1.0, 1.0),
                vec2(1.0, -1.0),
                
                vec2(1.0, 1.0),
                vec2(-1.0, 1.0),
                vec2(1.0, -1.0)
            );
            
            vec3 colors[6] = vec3[](
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
                vec3(0.0, 0.0, 1.0),
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

#[allow(clippy::needless_question_mark)]
mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            layout(location = 0) in vec3 fragColor;
            layout(location = 0) out vec4 f_color;

            layout(set = 0, binding = 0) uniform sampler2D tex;

            in vec4 gl_FragCoord;

            vec2 iResolution = vec2(800,600);

            float rand(vec2 co){
                return fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
            }

            void main() {
                vec2 uv = gl_FragCoord.xy / iResolution.xy;
                uv.y = 1.0-uv.y;  // flip
                f_color = texture(tex, uv);
            }
        "
    }
}
