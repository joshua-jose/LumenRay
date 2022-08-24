use std::fs::File;
use std::intrinsics::variant_count;
use std::io::prelude::*;

use winit::{
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, ModifiersState, MouseButton, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
};

use crate::{
    renderer::{CPURenderer, CameraComponent, TransformComponent},
    scene::Scene,
    vec3,
    vk::{BufferType, VkBackend, ELEM_PER_PIX},
    Vec3, Vec4,
};

const NUM_KEYS: usize = variant_count::<VirtualKeyCode>();
//const NUM_MOUSE_BUTTON: usize = variant_count::<MouseButton>();

//TODO: rename this
pub struct Engine {
    event_loop:  Option<EventLoop<()>>,
    backend:     VkBackend,
    renderer:    CPURenderer,
    framebuffer: Vec<Vec4>,
    width:       u32,
    height:      u32,

    keymap:         [bool; NUM_KEYS],
    modifiers:      ModifiersState,
    mouse_left:     bool,
    mouse_right:    bool,
    mouse_dx:       f32,
    mouse_dy:       f32,
    mouse_captured: bool,
}

impl Engine {
    pub fn new(width: u32, height: u32) -> Self {
        let event_loop = Some(EventLoop::new());
        let mut backend = VkBackend::new(event_loop.as_ref().unwrap(), "LumenRay", width, height);

        /*
        let vs = vs::load(backend.device.clone()).unwrap();
        let fs = fs::load(backend.device.clone()).unwrap();
        backend.streaming_setup(vs.entry_point("main").unwrap(), fs.entry_point("main").unwrap());

        // CPU local frame buffer

        */
        let framebuffer: Vec<Vec4> = vec![Vec4::splat(0.0); (width * height) as usize];
        let cs = cs::load(backend.device.clone()).unwrap();
        backend.compute_setup(cs.entry_point("main").unwrap());

        let renderer = CPURenderer::new();

        let window = backend.surface.window();
        window.set_cursor_grab(true).unwrap();
        window.set_cursor_visible(false);

        Self {
            event_loop,
            backend,
            renderer,
            framebuffer,
            width,
            height,

            keymap: [false; NUM_KEYS],
            modifiers: ModifiersState::empty(),
            mouse_left: false,
            mouse_right: false,
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            mouse_captured: true,
        }
    }

    pub fn get_texture_by_path(&mut self, path: &str, uscale: f32, vscale: f32) -> u32 {
        self.renderer.get_texture_by_path(path, uscale, vscale)
    }
    pub fn get_texture_by_colour(&mut self, colour: Vec3) -> u32 { self.renderer.get_texture_by_colour(colour) }

    pub fn run(mut self, mut scene: Scene) {
        let mut metric_file = File::create("metrics.csv").unwrap();
        let mut n: u32 = 0;

        let event_loop = self.event_loop.take().unwrap();

        event_loop.run(move |ev, _, control_flow| match ev {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::MainEventsCleared => self.on_window_update(&mut scene, &mut n, &mut metric_file),

            // keyboard input event
            Event::DeviceEvent {
                event:
                    DeviceEvent::Key(KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    }),
                ..
            } => self.on_key_down(key, state),

            Event::WindowEvent {
                event: WindowEvent::ModifiersChanged(modifiers),
                ..
            } => self.on_modifiers_changed(modifiers),

            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => self.on_mouse_down(button, state),

            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => self.on_mouse_move(delta),

            _ => (),
        });
    }

    fn render(&mut self, scene: &mut Scene, metric_file: &mut File) {
        let frame_start = std::time::Instant::now();

        /*
        self.renderer
            .draw(&mut self.framebuffer, self.width as usize, self.height as usize, scene);
        //debug!("Draw time: {:.2?}", frame_start.elapsed());

        let draw_time = frame_start.elapsed();

        let packet = draw_time.as_nanos().to_string() + "\n";
        metric_file.write_all(packet.as_bytes()).unwrap();
        //metric_file.write_all("\n".as_bytes()).unwrap();
        // metric_stream.write_all(&packet).unwrap();

        // reinterpet framebuffer as a slice of f32s
        let buffer_pix = unsafe {
            std::slice::from_raw_parts_mut(
                self.framebuffer.as_mut_ptr() as *mut BufferType,
                (self.width * self.height * ELEM_PER_PIX) as usize,
            )
        };
        //let now = std::time::Instant::now();
        self.backend.streaming_submit(buffer_pix);
        */
        self.backend.compute_submit();
        //debug!("Submit time: {:.2?}", now.elapsed());
        //debug!("Frame time: {:.2?}", frame_start.elapsed());
    }

    fn update(&mut self, scene: &mut Scene, n: &mut u32) {
        *n += 1;

        /*
        let light_pos = vec3(4.0 * (*n as f32 / 20.0).sin(), 2.0, -1.0);
        for (_, (light_t, _)) in scene.query_mut::<(&mut TransformComponent, &PointLightComponent)>() {
            light_t.position = light_pos;
        }
        */

        let mut offset = vec3(0.0, 0.0, 0.0);
        let delta: f32 = if self.modifiers.shift() { 0.2 } else { 0.1 };

        if self.keymap[VirtualKeyCode::W as usize] {
            offset.z += delta;
        }
        if self.keymap[VirtualKeyCode::A as usize] {
            offset.x -= delta;
        }
        if self.keymap[VirtualKeyCode::S as usize] {
            offset.z -= delta;
        }
        if self.keymap[VirtualKeyCode::D as usize] {
            offset.x += delta;
        }
        if self.keymap[VirtualKeyCode::Escape as usize] && self.mouse_captured {
            self.mouse_captured = false;
            let window = self.backend.surface.window();
            window.set_cursor_grab(false).unwrap();
            window.set_cursor_visible(true);
        }

        if self.mouse_left && !self.mouse_captured {
            self.mouse_captured = true;
            let window = self.backend.surface.window();
            window.set_cursor_grab(true).unwrap();
            window.set_cursor_visible(false);
        }

        for (_, (t, c)) in scene.query_mut::<(&mut TransformComponent, &mut CameraComponent)>() {
            if self.mouse_dx.abs() > 1.0 && self.mouse_captured {
                c.yaw += self.mouse_dx * 0.002;
            }
            if self.mouse_dy.abs() > 1.0 && self.mouse_captured {
                c.pitch += self.mouse_dy * 0.002;
            }
            t.position += c.get_rot_mat() * offset;
        }

        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
    }

    fn on_mouse_move(&mut self, delta: (f64, f64)) {
        self.mouse_dx += delta.0 as f32;
        self.mouse_dy += delta.1 as f32;
    }

    fn on_key_down(&mut self, key: VirtualKeyCode, state: ElementState) {
        self.keymap[key as usize] = state == ElementState::Pressed;
    }

    fn on_modifiers_changed(&mut self, modifiers: ModifiersState) { self.modifiers = modifiers; }

    fn on_mouse_down(&mut self, button: MouseButton, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        match button {
            MouseButton::Left => self.mouse_left = pressed,
            MouseButton::Right => self.mouse_right = pressed,
            MouseButton::Middle => (),
            MouseButton::Other(_) => (),
        };
    }

    fn on_window_update(&mut self, scene: &mut Scene, n: &mut u32, metric_file: &mut File) {
        self.update(scene, n);
        self.render(scene, metric_file);
    }
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

#[allow(clippy::needless_question_mark)]
mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path:"shaders/gpu_render.comp"
    }
}
