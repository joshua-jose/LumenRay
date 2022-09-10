// backend.compute_bind_buffer()

// potentially... :
// compute_add_pass

use std::{cell::RefCell, sync::Arc};

use crate::{
    rgb,
    scene::Scene,
    soft_blue,
    vk::{Buffer, DispatchSize, ImageArray, OutputImage, Sampler, Set, Shader, TextureArray, VkBackend},
    Mat4, Vec3,
};

use log::debug;
use render_mod::ty::{Plane, PointLight, Sphere};
use vulkano::sampler::{Filter, SamplerAddressMode, SamplerCreateInfo};

use super::{
    srgb_to_linear, CameraComponent, MaterialComponent, PlaneRenderComponent, PointLightComponent,
    SphereRenderComponent, Texture, TransformComponent,
};

pub struct GPURenderer {
    backend: Arc<RefCell<VkBackend>>,

    sphere_buffer: Arc<Buffer<Sphere>>,
    plane_buffer:  Arc<Buffer<Plane>>,
    lights_buffer: Arc<Buffer<PointLight>>,

    texture_paths: Vec<String>,
    albedo_array:  Arc<TextureArray>,
}

//TODO: report variable descriptor bug

#[allow(clippy::new_without_default)]
impl GPURenderer {
    pub fn new(backend: Arc<RefCell<VkBackend>>) -> Self {
        let output_image = OutputImage::new(backend.clone());

        //TODO: gen_buffers inconsistent
        let sphere_buffer = backend.borrow().gen_buffer(1);
        let plane_buffer = backend.borrow().gen_buffer(1);
        let lights_buffer = backend.borrow().gen_buffer(1);

        let tex_sampler = Arc::new(Sampler::new(
            backend.clone(),
            SamplerCreateInfo::simple_repeat_linear_no_mipmap(),
        ));
        let albedo_array = Arc::new(TextureArray::new(backend.clone()));

        let lm_sampler = Arc::new(Sampler::new(
            backend.clone(),
            SamplerCreateInfo {
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                ..Default::default()
            },
        ));
        // Create storage images for radiosity
        let current_emissives = Arc::new(ImageArray::new(backend.clone()));
        let new_emissives = Arc::new(ImageArray::new(backend.clone()));
        let lightmaps = Arc::new(ImageArray::new(backend.clone()));
        let sample_positions = Arc::new(ImageArray::new(backend.clone()));
        let sample_albedos = Arc::new(ImageArray::new(backend.clone()));
        let sample_normals = Arc::new(ImageArray::new(backend.clone()));

        let lm_width = 12 * 8;
        let lm_height = 12 * 8;
        current_emissives.push_images(lm_width, lm_height, 8);
        new_emissives.push_images(lm_width, lm_height, 8);
        lightmaps.push_images(lm_width, lm_height, 8);
        sample_positions.push_images(lm_width, lm_height, 8);
        sample_albedos.push_images(lm_width, lm_height, 8);
        sample_normals.push_images(lm_width, lm_height, 8);

        let render_mod = render_mod::load(backend.borrow().device.clone()).unwrap();
        let radiosity_mod = radiosity_mod::load(backend.borrow().device.clone()).unwrap();

        let render_shader_sets = [
            Set::new(&[
                output_image,
                sphere_buffer.clone(),
                plane_buffer.clone(),
                lights_buffer.clone(),
            ]),
            Set::new(&[tex_sampler.clone(), albedo_array.clone()]),
            Set::new(&[lm_sampler, new_emissives.clone()]), // FIXME:
        ];
        let render_shader = Shader::load_from_module(render_mod, &render_shader_sets, DispatchSize::FrameResolution);

        let radiosity_shader_sets = [
            Set::new(&[sphere_buffer.clone(), plane_buffer.clone(), lights_buffer.clone()]),
            Set::new(&[tex_sampler, albedo_array.clone()]),
            Set::new(&[current_emissives]),
            Set::new(&[new_emissives]),
            Set::new(&[lightmaps]),
            Set::new(&[sample_positions]),
            Set::new(&[sample_albedos]),
            Set::new(&[sample_normals]),
        ];
        let radiosity_shader = Shader::load_from_module(
            radiosity_mod,
            &radiosity_shader_sets,
            DispatchSize::Custom(lm_width, lm_height, 8),
        );

        backend
            .borrow_mut()
            .compute_setup(vec![radiosity_shader, render_shader]);

        let mut renderer = Self {
            backend,
            sphere_buffer,
            plane_buffer,
            lights_buffer,
            texture_paths: vec![],
            albedo_array,
        };

        renderer.get_texture_by_colour(soft_blue!());
        renderer
    }

    pub fn get_texture_by_path(&mut self, path: &str) -> u32 {
        if let Some(idx) = self.texture_paths.iter().position(|x| x == path) {
            idx as u32
        } else {
            debug!("Loading texture from path \"{}\"", path);
            let tex = Texture::from_path(path);
            self.albedo_array.push_texture(tex.width, tex.height, tex.data);

            self.texture_paths.push(path.to_owned());
            (self.texture_paths.len() - 1) as u32
        }
    }
    pub fn get_texture_by_colour(&mut self, colour: Vec3) -> u32 {
        let path = format!(
            "colour/{},{},{}",
            (colour.x * 1024.0).round() as u16,
            (colour.y * 1024.0).round() as u16,
            (colour.z * 1024.0).round() as u16
        );
        if let Some(idx) = self.texture_paths.iter().position(|x| x == &path) {
            idx as u32
        } else {
            let tex = Texture::from_colour_srgb(colour);
            self.albedo_array.push_texture(tex.width, tex.height, tex.data);

            self.texture_paths.push(path);
            (self.texture_paths.len() - 1) as u32
        }
    }

    pub fn draw(&mut self, scene: &mut Scene) {
        //TODO: move render scene stuff into here, it's redundant

        // get the first camera from the query
        let (_, (camera_transform, camera_component)) = scene
            .query_mut::<(&TransformComponent, &CameraComponent)>()
            .into_iter()
            .next()
            .unwrap();

        //let render_scene = scene.query_scene_objects();
        //let camera = render_scene.camera;

        let rot_mat = camera_component.get_rot_mat();
        let fov_deg: f32 = camera_component.fov;

        let camera_position = camera_transform.position.to_array();
        let camera_zdepth = (fov_deg * 0.5).to_radians().tan().recip();
        let camera_rotation = Mat4::from_mat3(rot_mat.transpose()).to_cols_array_2d();

        //TODO: materials are an index into another buffer

        let spheres = scene
            .query_mut::<(&TransformComponent, &SphereRenderComponent, &MaterialComponent)>()
            .into_iter()
            .map(|(_, (t, s, m))| Sphere {
                position: t.position.to_array(),
                radius: s.radius,
                mat: m.into(),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        let planes = scene
            .query_mut::<(&TransformComponent, &PlaneRenderComponent, &MaterialComponent)>()
            .into_iter()
            .map(|(_, (t, p, m))| Plane {
                position: t.position.to_array(),
                normal: p.normal.to_array(),
                tangent: p.tangent.to_array(),
                mat: m.into(),
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

        let mut backend = self.backend.borrow_mut();
        let mut builder = backend.compute_begin_submit();
        builder
            .add_shader_execution(0, Some(radiosity_mod::ty::Constants { stage: 0 }))
            .add_shader_execution(0, Some(radiosity_mod::ty::Constants { stage: 0 }))
            .add_shader_execution(
                1,
                Some(render_mod::ty::Constants {
                    camera_position,
                    camera_rotation,
                    camera_zdepth,
                }),
            );

        builder.submit();

        /*
        self.backend.borrow_mut().compute_submit(&[
            Some(radiosity_mod::ty::Constants { stage: 0 }),
            Some(render_mod::ty::Constants {
                camera_position,
                camera_rotation,
                camera_zdepth,
            }),
        ]);
        */
    }
}

impl From<&MaterialComponent> for render_mod::ty::Material {
    fn from(m: &MaterialComponent) -> Self {
        Self {
            tex_id: m.tex_id,
            tex_scale: m.tex_scale.to_array(),
            ambient: m.ambient,
            diffuse: m.diffuse,
            specular: m.specular,
            shininess: m.shininess,
            reflectivity: m.reflectivity,
            emissive: m.emissive,

            ..Default::default()
        }
    }
}

//TODO: get backend to deal with this (at runtime?)
#[allow(clippy::needless_question_mark)]
mod render_mod {

    vulkano_shaders::shader! {
        ty: "compute",
        path:"shaders/gpu_render.comp",
        exact_entrypoint_interface: false, // Stops it from analysing what descriptors are *actually* used
        types_meta: {use bytemuck::{Pod, Zeroable}; #[derive(Copy,Clone,Pod, Zeroable, Default, Debug)] impl crate::vk::BufferType},
    }
}

#[allow(clippy::needless_question_mark)]
mod radiosity_mod {

    vulkano_shaders::shader! {
        ty: "compute",
        path:"shaders/radiosity.comp",
        exact_entrypoint_interface: false, // Stops it from analysing what descriptors are *actually* used
        types_meta: {use bytemuck::{Pod, Zeroable}; #[derive(Copy,Clone,Pod, Zeroable, Default)] impl crate::vk::BufferType},
    }
}
