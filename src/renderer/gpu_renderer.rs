// backend.compute_bind_buffer()

// potentially... :
// compute_add_pass

use std::{cell::RefCell, sync::Arc};

use crate::{
    scene::Scene,
    vk::{Buffer, VkBackend},
    Mat4,
};

use cs::ty::{Material, Plane, PointLight, Sphere};

use super::{MaterialComponent, PlaneRenderComponent, PointLightComponent, SphereRenderComponent, TransformComponent};

pub struct GPURenderer {
    backend:       Arc<RefCell<VkBackend>>,
    sphere_buffer: Buffer<Sphere>,
    plane_buffer:  Buffer<Plane>,
    lights_buffer: Buffer<PointLight>,
}

#[allow(clippy::new_without_default)]
impl GPURenderer {
    pub fn new(backend: Arc<RefCell<VkBackend>>) -> Self {
        let sphere_buffer = backend.borrow().gen_buffer(1);
        let plane_buffer = backend.borrow().gen_buffer(1);
        let lights_buffer = backend.borrow().gen_buffer(1);

        let cs = cs::load(backend.borrow().device.clone()).unwrap();
        backend.borrow_mut().compute_setup(cs.entry_point("main").unwrap());

        Self {
            backend,
            sphere_buffer,
            plane_buffer,
            lights_buffer,
        }
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

        //TODO: materials are an index into another buffer

        let spheres = scene
            .query_mut::<(&TransformComponent, &SphereRenderComponent, &MaterialComponent)>()
            .into_iter()
            .map(|(_, (t, s, m))| Sphere {
                position: t.position.to_array(),
                radius: s.radius,
                mat: Material {
                    colour:       [0.7, 0.7, 0.7],
                    ambient:      m.ambient,
                    diffuse:      m.diffuse,
                    specular:     m.specular,
                    shininess:    m.shininess,
                    reflectivity: m.reflectivity,
                    emissive:     m.emissive,
                },
                ..Default::default()
            })
            .collect::<Vec<_>>();

        let planes = scene
            .query_mut::<(&TransformComponent, &PlaneRenderComponent, &MaterialComponent)>()
            .into_iter()
            .map(|(_, (t, p, m))| Plane {
                position: t.position.to_array(),
                normal: p.normal.to_array(),
                mat: Material {
                    colour:       p.normal.to_array(),
                    ambient:      m.ambient,
                    diffuse:      m.diffuse,
                    specular:     m.specular,
                    shininess:    m.shininess,
                    reflectivity: m.reflectivity,
                    emissive:     m.emissive,
                },
                ..Default::default()
            })
            .collect::<Vec<_>>();

        let lights = scene
            .query_mut::<(&TransformComponent, &PointLightComponent)>()
            .into_iter()
            .map(|(_, (t, p))| PointLight {
                position:  t.position.to_array(),
                intensity: p.intensity,
            })
            .collect::<Vec<_>>();

        self.sphere_buffer.write(&spheres);
        self.plane_buffer.write(&planes);
        self.lights_buffer.write(&lights);

        self.backend.borrow_mut().compute_submit(
            cs::ty::Constants {
                camera_position,
                camera_rotation,
                camera_zdepth,
            },
            &[&self.sphere_buffer, &self.plane_buffer, &self.lights_buffer],
        );
    }
}

#[allow(clippy::needless_question_mark)]
mod cs {

    vulkano_shaders::shader! {
        ty: "compute",
        path:"shaders/gpu_render.comp",
        types_meta: {use bytemuck::{Pod, Zeroable}; #[derive(Copy,Clone,Pod, Zeroable, Default)] impl crate::vk::BufferType},
    }
}
