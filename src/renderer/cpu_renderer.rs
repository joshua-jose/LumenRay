use std::intrinsics::unlikely;

use super::{MaterialComponent, Ray};
use crate::{
    scene::{RenderScene, Scene},
    vec3, Reflectable, Vec3, Vec3Swizzles, Vec4,
};
use rayon::prelude::*;

//TODO: move to own file / change implementation
pub struct HitInfo {
    pub object_idx: u32,
    pub position:   Vec3,
    pub normal:     Vec3,
}

pub struct CPURenderer {
    //scene:       Scene, // or take the hecs::world directly if it's more performant (but less desirable)
}

static mut N: i32 = 0;
const NO_HIT: f32 = f32::MAX; // value to return when no intersection
const SMALL_DISTANCE: f32 = 0.0001;

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

        unsafe { N += 1 };
        // TODO: Add to scene
        let light_pos = vec3(4.0 * (unsafe { N as f32 / 20.0 }).sin(), 2.0, -1.0);
        let camera_pos = vec3(0.0, 0.0, -5.0);

        let fov_deg: f32 = 90.0;
        let zdepth = (fov_deg * 0.5).to_radians().tan().recip();

        //TODO: pass a struct of query results to ray casting function in future
        //TODO: Maybe abtract this into a `Hittable`, but that would probably balloon memory usage
        let mut render_scene = scene.query_scene_objects();
        render_scene.light_pos = light_pos;

        framebuffer.par_iter_mut().enumerate().for_each(|(i, pix)| {
            // generate direction vectors from screen space UV coords
            let x = i % width;
            let y = i / width;

            let u = (x as f32 - (0.5 * (width as f32))) / height as f32; // divide u by height to account for aspect ratio
            let v = (y as f32 - (0.5 * (height as f32))) / height as f32;
            let direction = vec3(u, v, zdepth);

            let col = Self::cast_sight_ray(
                Ray {
                    origin: camera_pos,
                    direction,
                },
                &render_scene,
            );
            *pix = col.xyzz();
        });
    }

    //TODO: rename this to something different to the raw cast_ray
    fn cast_sight_ray(mut ray: Ray, render_scene: &RenderScene) -> Vec3 {
        let sky_colour = Vec3::splat(0.2);

        let h = Self::cast_ray(&mut ray, render_scene); // calculate whether this ray hits any scene geometry

        //TODO: multiple lights
        match h {
            Some(info) => Self::shade_object(&mut ray, info, render_scene),
            None => sky_colour,
        }
    }

    fn shade_object(ray: &mut Ray, info: HitInfo, render_scene: &RenderScene) -> Vec3 {
        let vec_to_light = render_scene.light_pos - info.position;
        let normal = info.normal;
        let direction = ray.direction;
        let position = info.position;

        //TODO: find another way to do this
        let sphere = &render_scene.spheres[info.object_idx as usize];
        let obj_col = sphere.material.colour;

        // Cheapo shadow hit calculation
        //TODO: proper soft shadows
        //TODO: replace constant
        let mut shadow_ray = Ray::new(info.position + (SMALL_DISTANCE * normal), vec_to_light);
        let shadow_h = Self::cast_ray(&mut shadow_ray, render_scene);
        let shade = match shadow_h {
            Some(_) => 0.1,
            None => 1.0,
        };

        let mut col = shade * obj_col * Self::phong(normal, vec_to_light, direction, 10.0, sphere.material);

        if sphere.material.reflectivity > 1e-3 {
            // very cheap fresnel effect
            let fresnel = (1.0 - normal.dot(-direction)).clamp(0.0, 1.0).powi(5);

            let reflection_vector = direction.reflect(normal);
            let reflection_colour = Self::cast_sight_ray(
                Ray {
                    origin:    position + (reflection_vector * SMALL_DISTANCE),
                    direction: reflection_vector,
                },
                render_scene,
            );

            col += (fresnel + 1.0).clamp(0.0, 1.0) * reflection_colour/*  * obj_col */;
        }
        col
    }

    #[inline]
    //TODO: Blinn Phong?
    fn phong(normal: Vec3, vec_to_light: Vec3, view_dir: Vec3, light_intensity: f32, mat: &MaterialComponent) -> f32 {
        let shininess = 4.0;

        let distance_to_light_sqd = vec_to_light.length_squared();
        let light_intensity = light_intensity / distance_to_light_sqd; // k/d^2
        let vec_to_light_norm = vec_to_light / distance_to_light_sqd.sqrt(); // normalize vector
        let light_reflection_vector = vec_to_light_norm.reflect(normal);

        // Phong shading algorithm
        let diffuse = mat.diffuse * light_intensity * vec_to_light_norm.dot(normal).max(0.0);

        let specular = if diffuse > 0.0 {
            mat.specular * light_intensity * light_reflection_vector.dot(view_dir).max(0.0).powf(shininess)
        } else {
            0.0
        };

        mat.ambient + diffuse + specular
    }

    // Takes a ray through a scene, and does the raw hit detection, returning what it hit and where.
    fn cast_ray(ray: &mut Ray, render_scene: &RenderScene) -> Option<HitInfo> {
        //TODO: if there are multiple spheres in the scene, calculate with SoA(structure of arrays) approach?

        // go through the scene, find the smallest distance
        let mut distance = NO_HIT;
        let mut nearest_entity: usize = usize::MAX;

        for (i, s) in render_scene.spheres.iter().enumerate() {
            let obj_distance = Self::ray_sphere_intersect(ray, s.transform.position, s.render.radius);
            if obj_distance < distance {
                distance = obj_distance;
                nearest_entity = i;
            };
        }

        if distance != NO_HIT {
            let position = ray.origin + (distance * ray.direction.normalize());
            let sphere_pos = render_scene.spheres[nearest_entity].transform.position;
            let normal = (position - sphere_pos).normalize();
            Some(HitInfo {
                object_idx: nearest_entity as u32,
                position,
                normal,
            })
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
}
