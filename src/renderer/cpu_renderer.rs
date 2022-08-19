use std::intrinsics::unlikely;

use super::{MaterialComponent, Ray};
use crate::{
    scene::{ObjectType, RenderScene, Scene},
    vec3, Reflectable, Vec3, Vec3Swizzles, Vec4,
};
use rayon::prelude::*;

//TODO: move to own file / change implementation
//TODO: simplify render pipeline? It's quite hard to add new types of objects
pub struct HitInfo<'a> {
    pub position: Vec3,
    pub normal:   Vec3,
    pub mat:      &'a MaterialComponent,
}

pub struct CPURenderer {
    //scene:       Scene, // or take the hecs::world directly if it's more performant (but less desirable)
}

const NO_HIT: f32 = f32::MAX; // value to return when no intersection
const SMALL_DISTANCE: f32 = 0.0001;
const MAX_BOUNCES: u32 = 2;

#[allow(clippy::new_without_default)] // default construction doesn't make sense here
impl CPURenderer {
    pub fn new() -> Self { Self {} }

    //TODO: could pass vec, width and height as framebuffer obj.
    pub fn draw(&self, framebuffer: &mut Vec<Vec4>, width: usize, height: usize, scene: &mut Scene) {
        //TODO: replace with actual camera object
        // generate rays based on camera info
        // ....
        // let origin = camera.position;
        // let width, height = camera.viewport_width, camera.viewport_height
        // let rot_matrix = camera.get_rotation_matrix()
        // let fov = camera.get_fov()

        //TODO: Maybe abtract this into a `Hittable`, but that would probably balloon memory usage

        let render_scene = scene.query_scene_objects();

        let camera = render_scene.camera;
        let camera_pos = camera.transform.position;
        let rot_mat = camera.camera.get_rot_mat();

        let fov_deg: f32 = camera.camera.fov;
        let zdepth = (fov_deg * 0.5).to_radians().tan().recip();

        //let now = std::time::Instant::now();
        framebuffer.par_iter_mut().enumerate().for_each(|(i, pix)| {
            // generate direction vectors from screen space UV coords
            let x = i % width;
            let y = i / width;

            let u = (x as f32 - (0.5 * (width as f32))) / height as f32; // divide u by height to account for aspect ratio
            let v = (y as f32 - (0.5 * (height as f32))) / height as f32;
            let direction = rot_mat * vec3(u, v, zdepth)/* .normalize() */;

            let col = Self::cast_sight_ray(
                Ray {
                    origin: camera_pos,
                    direction,
                },
                &render_scene,
                0,
            );
            *pix = col.xyzz();
        });
        //println!("Cast time: {:.2?}", now.elapsed());
    }

    //TODO: rename this to something different to the raw cast_ray
    fn cast_sight_ray(mut ray: Ray, render_scene: &RenderScene, depth: u32) -> Vec3 {
        let sky_colour = Vec3::splat(0.2);

        if depth >= MAX_BOUNCES {
            return sky_colour;
        };

        let h = Self::cast_ray(&mut ray, render_scene); // calculate whether this ray hits any scene geometry

        //TODO: multiple lights, implement as component
        match h {
            Some(info) => Self::shade_object(&mut ray, info, render_scene, depth),
            None => sky_colour,
        }
    }

    fn shade_object(ray: &mut Ray, info: HitInfo, render_scene: &RenderScene, depth: u32) -> Vec3 {
        let vec_to_light = render_scene.light.transform.position - info.position;
        let light_intensity = render_scene.light.light.intensity;
        let direction = ray.direction;
        let position = info.position;
        let normal = info.normal;
        let material = info.mat;

        let obj_col = material.colour;

        // Cheapo shadow hit calculation
        //TODO: proper soft shadows
        let mut shadow_ray = Ray::new(info.position + (SMALL_DISTANCE * normal), vec_to_light);
        let shadow_h = Self::cast_ray(&mut shadow_ray, render_scene);
        let shade = match shadow_h {
            Some(shadow_info) => {
                if (shadow_info.position - shadow_ray.origin).length_squared() > vec_to_light.length_squared() {
                    1.0
                } else {
                    0.1
                }
            }
            None => 1.0,
        };

        let mut col = obj_col
            * (material.ambient + shade * Self::phong(normal, vec_to_light, direction, light_intensity, material));

        if material.reflectivity > 1e-3 {
            // very cheap fresnel effect
            let fresnel = (1.0 - normal.dot(-direction)).clamp(0.0, 1.0).powi(5);

            let reflection_vector = direction.reflect(normal);
            let reflection_colour = Self::cast_sight_ray(
                Ray {
                    origin:    position + (reflection_vector * SMALL_DISTANCE * 3.0),
                    direction: reflection_vector,
                },
                render_scene,
                depth + 1,
            );

            col += (fresnel + material.reflectivity).clamp(0.0, 1.0) * reflection_colour * obj_col;
        }
        col
    }

    #[inline]
    //TODO: Blinn Phong?
    fn phong(normal: Vec3, vec_to_light: Vec3, view_dir: Vec3, light_intensity: f32, mat: &MaterialComponent) -> f32 {
        let distance_to_light_sqd = vec_to_light.length_squared();
        let light_intensity = light_intensity / distance_to_light_sqd; // k/d^2
        let vec_to_light_norm = vec_to_light / distance_to_light_sqd.sqrt(); // normalize vector
        let light_reflection_vector = vec_to_light_norm.reflect(normal);

        // Phong shading algorithm
        let diffuse = vec_to_light_norm.dot(normal).max(0.0);

        let specular = if diffuse > 0.0 {
            light_reflection_vector.dot(view_dir).max(0.0).powf(mat.shininess)
        } else {
            0.0
        };
        // TODO: split diffuse and specular so we can conditionally tint it
        light_intensity * (mat.diffuse * diffuse + mat.specular * specular)
    }

    // Takes a ray through a scene, and does the raw hit detection, returning what it hit and where.
    fn cast_ray<'a>(ray: &mut Ray, render_scene: &'a RenderScene<'a>) -> Option<HitInfo<'a>> {
        //TODO: if there are multiple spheres in the scene, calculate with SoA(structure of arrays) approach?

        // go through the scene, find the smallest distance
        let mut distance = NO_HIT;
        let mut nearest_entity = u32::MAX;
        let mut nearest_type = ObjectType::None;

        for (id, s) in &render_scene.spheres {
            let obj_distance = Self::ray_sphere_intersect(ray, s.transform.position, s.render.radius);
            if obj_distance < distance {
                distance = obj_distance;
                nearest_entity = *id;
                nearest_type = ObjectType::Sphere;
            };
        }

        for (id, p) in &render_scene.planes {
            let obj_distance = Self::ray_plane_intersect(ray, p.transform.position, p.render.normal);
            if obj_distance < distance {
                distance = obj_distance;
                nearest_entity = *id;
                nearest_type = ObjectType::Plane;
            };
        }

        if distance != NO_HIT {
            let position = ray.origin + (distance * ray.direction);

            let normal;
            let mat;
            match nearest_type {
                ObjectType::Sphere => {
                    let sphere = render_scene.get_sphere_by_id(nearest_entity).unwrap();

                    normal = (position - sphere.transform.position).normalize();
                    mat = sphere.material;
                }
                ObjectType::Plane => {
                    let plane = render_scene.get_plane_by_id(nearest_entity).unwrap();
                    normal = plane.render.normal;
                    mat = plane.material;
                }
                _ => unreachable!(),
            };

            Some(HitInfo { position, normal, mat })
        } else {
            None
        }
    }

    #[inline]
    fn ray_sphere_intersect(ray: &mut Ray, sphere_pos: Vec3, sphere_radius: f32) -> f32 {
        // quadratic formula constants for line-sphere intersection

        // vector from sphere center to ray origin.
        let c_to_o = ray.origin - sphere_pos;

        let a = ray.direction.length_squared();
        let b = 2.0 * (ray.direction.dot(c_to_o));
        let c = (c_to_o).length_squared() - sphere_radius.powi(2); //TODO: we could precompute radius sqd

        // distance of two intersection points
        let mut d0: f32 = NO_HIT;
        let mut d1: f32;

        // b^2 - 4ac
        let discrim = b.powi(2) - (4.0 * a * c);

        if discrim >= 0.0 {
            // now solve the quadratic, using a more stable computer friendly formula

            // if the discrim is close to 0, use a faster formula ignoring discrim.
            // This can be optimised by being looser on what is "close" to zero.
            if unlikely(discrim.abs() <= f32::EPSILON) {
                d0 = -0.5 * b / a;
                d1 = d0;
            } else {
                let sqrt_discrim = discrim.sqrt();
                let q = -0.5 * (b + (b.signum() * sqrt_discrim));
                d0 = q / a;
                d1 = c / q;
            }

            if d0 > d1 {
                (d0, d1) = (d1, d0);
            };

            // negative distances mean we intersect behind, we want d0 to be the positive intersection
            if unlikely(d0 < 0.0) {
                d0 = d1;
                if unlikely(d0 < 0.0) {
                    d0 = NO_HIT;
                }
            }
        }

        d0
    }

    #[inline]
    fn ray_plane_intersect(ray: &mut Ray, plane_pos: Vec3, plane_normal: Vec3) -> f32 {
        let denom = -plane_normal.dot(ray.direction);

        //TODO: constant?

        if denom > 1e-6 {
            let to_plane = plane_pos - ray.origin;
            let d = to_plane.dot(-plane_normal) / denom;
            if d >= 0.0 {
                d
            } else {
                NO_HIT
            }
        } else {
            NO_HIT
        }
    }
}
