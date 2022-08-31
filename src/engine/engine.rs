use std::intrinsics::variant_count;
use std::sync::Arc;
use std::{cell::RefCell, fs::File};

use winit::{
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, ModifiersState, MouseButton, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
};

use crate::{
    renderer::{CameraComponent, GPURenderer, TransformComponent},
    scene::Scene,
    vec3,
    vk::VkBackend,
    Vec3,
};

const NUM_KEYS: usize = variant_count::<VirtualKeyCode>();
//const NUM_MOUSE_BUTTON: usize = variant_count::<MouseButton>();

//TODO: rename this
pub struct Engine {
    event_loop: Option<EventLoop<()>>,
    backend:    Arc<RefCell<VkBackend>>,
    renderer:   GPURenderer,

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
        let backend = VkBackend::new(event_loop.as_ref().unwrap(), "LumenRay", width, height);

        let window = backend.surface.window();
        window.set_cursor_grab(true).unwrap();
        window.set_cursor_visible(false);

        let backend = Arc::new(RefCell::new(backend));
        let renderer = GPURenderer::new(backend.clone());

        Self {
            event_loop,
            backend,
            renderer,

            keymap: [false; NUM_KEYS],
            modifiers: ModifiersState::empty(),
            mouse_left: false,
            mouse_right: false,
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            mouse_captured: true,
        }
    }

    pub fn get_texture_by_path(&mut self, path: &str) -> u32 { self.renderer.get_texture_by_path(path) }
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
        //let frame_start = std::time::Instant::now();

        self.renderer.draw(scene);

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
        let backend = self.backend.borrow();

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
            let window = backend.surface.window();
            window.set_cursor_grab(false).unwrap();
            window.set_cursor_visible(true);
        }

        if self.mouse_left && !self.mouse_captured {
            self.mouse_captured = true;

            let window = backend.surface.window();
            window.set_cursor_grab(true).unwrap();
            window.set_cursor_visible(false);
        }

        let mut fov_add = 0.0;
        if self.mouse_right {
            fov_add = 1.0;
        }

        for (_, (t, c)) in scene.query_mut::<(&mut TransformComponent, &mut CameraComponent)>() {
            if self.mouse_dx.abs() > 1.0 && self.mouse_captured {
                c.yaw += self.mouse_dx * 0.002;
            }
            if self.mouse_dy.abs() > 1.0 && self.mouse_captured {
                c.pitch += self.mouse_dy * 0.002;
            }
            c.fov -= fov_add;
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

//TODO: get backend to deal with this (at runtime?)
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
