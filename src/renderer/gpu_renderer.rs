// backend.compute_bind_buffer()

// potentially... :
// compute_add_pass

use std::{cell::RefCell, sync::Arc};

use crate::{
    renderer::Mesh,
    rgb,
    scene::Scene,
    soft_blue,
    vk::{Buffer, DispatchSize, HasDescriptor, ImageArray, OutputImage, Sampler, Set, Shader, TextureArray, VkBackend},
    Mat4, Vec3,
};

use log::debug;
use render_mod::ty::{MeshInstance, Plane, PointLight, Sphere, Triangle, Vertex};
use vulkano::sampler::{Filter, SamplerAddressMode, SamplerCreateInfo};

use super::{
    srgb_to_linear, CameraComponent, MaterialComponent, MeshRenderComponent, PlaneRenderComponent, PointLightComponent,
    SphereRenderComponent, Texture, TransformComponent,
};

const RESOLUTION_U: u32 = 2;
const RESOLUTION_V: u32 = 2;
const LM_WIDTH: u32 = 18 * RESOLUTION_U;
const LM_HEIGHT: u32 = 18 * RESOLUTION_V;

pub struct GPURenderer {
    backend: Arc<RefCell<VkBackend>>,

    sphere_buffer: Arc<Buffer<Sphere>>,
    plane_buffer:  Arc<Buffer<Plane>>,
    lights_buffer: Arc<Buffer<PointLight>>,

    texture_paths: Vec<String>,
    albedo_array:  Arc<TextureArray>,

    mesh_paths:           Vec<String>,
    meshes:               Vec<Mesh>,
    vertex_buffer:        Arc<Buffer<Vertex>>,
    triangle_buffer:      Arc<Buffer<Triangle>>,
    mesh_instance_buffer: Arc<Buffer<MeshInstance>>,

    radiosity_computed: bool,
    current_emissives:  Arc<ImageArray>,
    new_emissives:      Arc<ImageArray>,
    lightmaps:          Arc<ImageArray>,
    sample_positions:   Arc<ImageArray>,
    sample_albedos:     Arc<ImageArray>,
    sample_normals:     Arc<ImageArray>,
    sample_sizes:       Arc<ImageArray>,
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

        let vertex_buffer = backend.borrow().gen_buffer(1);
        let triangle_buffer = backend.borrow().gen_buffer(1);
        let mesh_instance_buffer = backend.borrow().gen_buffer(1);

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
        let sample_sizes = Arc::new(ImageArray::new(backend.clone()));

        let render_mod = render_mod::load(backend.borrow().device.clone()).unwrap();
        let radiosity_mod = radiosity_mod::load(backend.borrow().device.clone()).unwrap();

        let render_shader_sets = [
            Set::new(&[
                output_image,
                sphere_buffer.clone(),
                plane_buffer.clone(),
                lights_buffer.clone(),
                vertex_buffer.clone(),
                triangle_buffer.clone(),
                mesh_instance_buffer.clone(),
            ]),
            Set::new(&[tex_sampler.clone(), albedo_array.clone()]),
            Set::new(&[lm_sampler, lightmaps.clone()]),
        ];
        let render_shader = Shader::load_from_module(render_mod, &render_shader_sets);

        let radiosity_shader_sets = [
            Set::new(&[
                sphere_buffer.clone(),
                plane_buffer.clone(),
                lights_buffer.clone(),
                vertex_buffer.clone(),
                triangle_buffer.clone(),
                mesh_instance_buffer.clone(),
            ]),
            Set::new(&[tex_sampler, albedo_array.clone()]),
            Set::new(&[current_emissives.clone()]),
            Set::new(&[new_emissives.clone()]),
            Set::new(&[lightmaps.clone()]),
            Set::new(&[sample_positions.clone()]),
            Set::new(&[sample_albedos.clone()]),
            Set::new(&[sample_normals.clone()]),
            Set::new(&[sample_sizes.clone()]),
        ];
        let radiosity_shader = Shader::load_from_module(radiosity_mod, &radiosity_shader_sets);

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

            mesh_paths: vec![],
            meshes: vec![],
            vertex_buffer,
            triangle_buffer,
            mesh_instance_buffer,

            radiosity_computed: false,
            current_emissives,
            new_emissives,
            lightmaps,
            sample_positions,
            sample_albedos,
            sample_normals,
            sample_sizes,
        };

        renderer.get_texture_by_colour(soft_blue!());
        renderer
    }

    fn add_object_lightmap(&mut self, width: u32, height: u32) {
        self.current_emissives.push_image(width, height);
        self.new_emissives.push_image(width, height);
        self.lightmaps.push_image(width, height);
        self.sample_positions.push_image(width, height);
        self.sample_albedos.push_image(width, height);
        self.sample_normals.push_image(width, height);
        self.sample_sizes.push_image(width, height);
    }

    fn get_lightmap_len(&self) -> u32 { self.lightmaps.variable_descriptor_count() }

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

    pub fn get_mesh_by_path(&mut self, path: &str) -> u32 {
        if let Some(idx) = self.mesh_paths.iter().position(|x| x == path) {
            idx as u32
        } else {
            debug!("Loading mesh from path \"{}\"", path);

            let mesh = Mesh::from_path(path);
            self.meshes.push(mesh);

            //TODO: We are reuploading all vertices everytime a mesh is added, a bit inefficient
            let vertices = self
                .meshes
                .iter()
                .flat_map(|m| m.vertices.iter().map(|v| v.into()).collect::<Vec<_>>())
                .collect::<Vec<_>>();
            self.vertex_buffer.write(&vertices);

            let triangles = self
                .meshes
                .iter()
                .flat_map(|m| {
                    m.triangles
                        .iter()
                        .map(|t| Triangle {
                            v1_idx: t.v1_idx,
                            v2_idx: t.v2_idx,
                            v3_idx: t.v3_idx,
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            self.triangle_buffer.write(&triangles);

            self.mesh_paths.push(path.to_owned());
            (self.mesh_paths.len() - 1) as u32
        }
    }

    pub fn draw(&mut self, scene: &mut Scene) {
        // get the first camera from the query
        let (_, (camera_transform, camera_component)) = scene
            .query_mut::<(&TransformComponent, &CameraComponent)>()
            .into_iter()
            .next()
            .unwrap();

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
                width: p.width,
                height: p.height,
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

        let mesh_instances = scene
            .query_mut::<(&TransformComponent, &MeshRenderComponent, &MaterialComponent)>()
            .into_iter()
            .map(|(_, (t, mesh, mat))| {
                //TODO: Model space to world space transform
                // Probably upload a mesh buffer which has positions, scale, lm_id
                // also eventually unify all id's

                let mut start_triangle_idx = 0;
                let mut start_vertex_idx = 0;
                for i in 0..mesh.mesh_id {
                    start_triangle_idx += self.meshes[i as usize].len_triangles();
                    start_vertex_idx += self.meshes[i as usize].len_vertices();
                }
                MeshInstance {
                    position: t.position.to_array(),
                    start_triangle_idx,
                    start_vertex_idx,
                    num_triangles: self.meshes[mesh.mesh_id as usize].len_triangles(),
                    mat: mat.into(),
                }
            })
            .collect::<Vec<_>>();

        if !spheres.is_empty() {
            self.sphere_buffer.write(&spheres);
        }
        self.plane_buffer.write(&planes);
        self.lights_buffer.write(&lights);
        if !mesh_instances.is_empty() {
            self.mesh_instance_buffer.write(&mesh_instances);
        }

        //TODO: broken if there are only planes in scene
        //TODO: probably fixed by sending over object ids

        //TODO: if objects are added or removed then they won't necessarily have the right sized lightmap
        // due to the inability to remove or resize lightmaps at a position.
        let num_lightmaps = self.get_lightmap_len() as usize;
        let num_spheres = spheres.len();
        let num_planes = planes.len();
        let num_mesh_instances = mesh_instances.len();
        let num_objs = num_spheres + num_planes + num_mesh_instances;
        if num_objs > num_lightmaps {
            let delta = num_objs - num_lightmaps;
            for i in 0..delta {
                if i < num_spheres {
                    // TODO: correct resolution
                    self.add_object_lightmap(6 * RESOLUTION_U, 6 * RESOLUTION_V);
                } else if i < (num_spheres + num_planes) {
                    let idx = i - num_spheres;
                    let width = planes[idx].width;
                    let height = planes[idx].height;
                    self.add_object_lightmap(width.ceil() as u32 * RESOLUTION_U, height.ceil() as u32 * RESOLUTION_V);
                } else if i < (num_spheres + num_planes + num_mesh_instances) {
                    self.add_object_lightmap(32 * RESOLUTION_U, 32 * RESOLUTION_V);
                }
            }
        }

        let mut backend = self.backend.borrow_mut();
        let mut builder = backend.compute_begin_submit();
        if !self.radiosity_computed {
            self.radiosity_computed = true;

            let dispatch_size = DispatchSize::Custom(LM_WIDTH, LM_HEIGHT, num_objs as u32); //TODO: correct dispatch size

            builder
                .add_shader_execution(0, dispatch_size, Some(radiosity_mod::ty::Constants { stage: 0 }))
                .add_shader_execution(0, dispatch_size, Some(radiosity_mod::ty::Constants { stage: 1 }))
                .add_shader_execution(0, dispatch_size, Some(radiosity_mod::ty::Constants { stage: 2 }));
            //.add_shader_execution(0, dispatch_size, Some(radiosity_mod::ty::Constants { stage: 1 }))
            //.add_shader_execution(0, dispatch_size, Some(radiosity_mod::ty::Constants { stage: 2 }));
        }
        builder.add_shader_execution(
            1,
            DispatchSize::FrameResolution,
            Some(render_mod::ty::Constants {
                camera_position,
                camera_rotation,
                camera_zdepth,
            }),
        );

        builder.submit();
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

impl From<&super::Vertex> for render_mod::ty::Vertex {
    fn from(v: &super::Vertex) -> Self {
        Self {
            position: v.position.to_array(),
            normal: v.normal.to_array(),
            uv: v.uv.to_array(),
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
