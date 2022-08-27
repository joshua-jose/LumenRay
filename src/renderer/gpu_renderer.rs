// backend.compute_bind_buffer()

// potentially... :
// compute_add_pass

use std::{cell::RefCell, sync::Arc};

use crate::{
    scene::Scene,
    vk::{Buffer, VkBackend},
    Mat4,
};

use cs::ty::{Material, Sphere};

pub struct GPURenderer {
    backend:       Arc<RefCell<VkBackend>>,
    sphere_buffer: Buffer<Sphere>,
}

#[allow(clippy::new_without_default)]
impl GPURenderer {
    pub fn new(backend: Arc<RefCell<VkBackend>>) -> Self {
        let sphere_buffer = backend.borrow().gen_buffer(1);

        let cs = cs::load(backend.borrow().device.clone()).unwrap();
        backend.borrow_mut().compute_setup(cs.entry_point("main").unwrap());

        Self { backend, sphere_buffer }
    }

    pub fn draw(&mut self, scene: &mut Scene) {
        //TODO: move render scene stuff into here, it's redundant
        let render_scene = scene.query_scene_objects();

        let camera = render_scene.camera;

        let rot_mat = camera.camera.get_rot_mat();
        let fov_deg: f32 = camera.camera.fov;

        let camera_position = camera.transform.position.to_array();
        let camera_zdepth = (fov_deg * 0.5).to_radians().tan().recip();
        let camera_rotation = Mat4::from_mat3(rot_mat.transpose()).to_cols_array_2d();

        //TODO: index materials
        let data = [
            Sphere {
                position: [0.0, 0.0, 0.0],
                radius:   1.0,
                mat:      Material {
                    colour:       [1.0, 0.0, 0.0],
                    ambient:      0.2,
                    diffuse:      1.0,
                    specular:     0.8,
                    shininess:    32.0,
                    reflectivity: 1.0,
                    emissive:     0.0,
                },
                _dummy0:  Default::default(),
            },
            Sphere {
                position: [2.0, 0.0, 0.0],
                radius:   1.0,
                mat:      Material {
                    colour:       [1.0, 0.0, 0.0],
                    ambient:      0.2,
                    diffuse:      1.0,
                    specular:     0.8,
                    shininess:    32.0,
                    reflectivity: 1.0,
                    emissive:     0.0,
                },
                _dummy0:  Default::default(),
            },
        ];
        self.sphere_buffer.write(&data);
        //TODO: replace with backend.compute_bind_buffer()
        self.backend.borrow_mut().compute_submit(
            cs::ty::Constants {
                camera_position,
                camera_rotation,
                camera_zdepth,
            },
            &[&self.sphere_buffer],
        );
    }
}

#[allow(clippy::needless_question_mark)]
mod cs {

    vulkano_shaders::shader! {
        ty: "compute",
        path:"shaders/gpu_render.comp",
        types_meta: {use bytemuck::{Pod, Zeroable}; #[derive(Copy,Clone,Pod, Zeroable)] impl crate::vk::BufferType},
    }
}
