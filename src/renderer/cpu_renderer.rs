use std::{cell::RefCell, intrinsics::unlikely};

use super::Ray;
use crate::{
    scene::{Query, Scene, SphereRenderQuery},
    vec3, Reflectable, Vec3, Vec3Swizzles, Vec4,
};
use rayon::prelude::*;

//TODO: move to own file / change implementation
pub struct HitInfo {
    pub object_id: u32,
    pub position:  Vec3,
    pub direction: Vec3,
    pub normal:    Vec3,
}

pub struct CPURenderer<'a> {
    //scene:       Scene, // or take the hecs::world directly if it's more performant (but less desirable)
    sphere_query: RefCell<Query<SphereRenderQuery<'a>>>,
}

// on draw, create direction vectors from transform

static mut N: i32 = 0;
const NO_HIT: f32 = f32::MAX; // value to return when no intersection

#[allow(clippy::new_without_default)] // default construction doesn't make sense here
impl CPURenderer<'_> {
    pub fn new() -> Self {
        Self {
            sphere_query: RefCell::new(Query::default()),
        }
    }

    pub fn draw(&self, framebuffer: &mut Vec<Vec4>, width: usize, height: usize, scene: &mut Scene) {
        //TODO: could generate these as we iterate through the framebuffer
        let hits = self.cast_sight_rays(width, height, scene);

        unsafe { N += 1 };
        // TODO: Add to scene
        let light_pos = vec3(4.0 * (unsafe { N as f32 / 20.0 }).sin(), 2.0, -1.0);
        let sky_colour = Vec4::splat(0.2);

        framebuffer.par_iter_mut().enumerate().for_each(|(i, pix)| {
            let h = &hits[i];
            if h.is_some() {
                let info = h.as_ref().unwrap();

                // calculate phong shading on point

                let col = Self::phong(
                    vec3(0.0, 0.0, 1.0),
                    info.normal,
                    light_pos - info.position,
                    info.direction,
                    20.0,
                    0.1,
                    1.0,
                    0.0,
                );

                *pix = col.xyzz();
            } else {
                *pix = sky_colour;
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    //TODO: should take mat argument instead
    //TODO: Blinn Phong?
    fn phong(
        col: Vec3, normal: Vec3, vec_to_light: Vec3, view_dir: Vec3, light_intensity: f32, mat_ambient: f32,
        mat_diffuse: f32, mat_specular: f32,
    ) -> Vec3 {
        let shininess = 4.0;

        let distance_to_light_sqd = vec_to_light.length_squared();
        let light_intensity = light_intensity / distance_to_light_sqd; // k/d^2
        let vec_to_light_norm = vec_to_light / distance_to_light_sqd.sqrt(); // normalize vector
        let light_reflection_vector = vec_to_light_norm.reflect(normal);

        // Phong shading algorithm
        let diffuse = mat_diffuse * light_intensity * vec_to_light_norm.dot(normal).max(0.0);

        let specular = if diffuse > 0.0 {
            mat_specular * light_intensity * light_reflection_vector.dot(view_dir).max(0.0).powf(shininess)
        } else {
            0.0
        };

        (mat_ambient + diffuse + specular) * col
    }

    #[allow(clippy::uninit_vec)]
    pub fn cast_sight_rays(
        &self, /* , camera: &Camera*/
        width: usize, height: usize, scene: &mut Scene,
    ) -> Vec<Option<HitInfo>> {
        //TODO: replace with actual camera object
        // generate rays based on camera info
        // ....
        // let origin = camera.position;
        // let width, height = camera.viewport_width, camera.viewport_height
        // let rot_matrix = camera.get_rotation_matrix()
        // let fov = camera.get_fov()

        let mut hits = Vec::with_capacity(width * height);
        unsafe {
            // we will not read from any of these locations until we have written them! This is safe, and so much faster than pre-initializing them.
            hits.set_len(width * height);
        }

        let camera_pos = vec3(0.0, 0.0, -5.0);

        // Query objects in the scene
        //TODO: pass a struct of query results to ray casting function in future
        let mut query = self.sphere_query.borrow_mut();
        let sphere_res = query.query(scene).unwrap().map(|(_, s)| s).collect::<Vec<_>>();

        // use par_iter_mut to calculate across all cores
        hits.par_iter_mut().enumerate().for_each(|(i, h)| {
            // generate direction vectors from screen space UV coords
            let x = i % width;
            let y = i / width;

            let u = (x as f32 - (0.5 * (width as f32))) / height as f32; // divide u by height to account for aspect ratio
            let v = (y as f32 - (0.5 * (height as f32))) / height as f32;
            let direction = vec3(u, v, 1.0);

            let mut r = Ray::new(camera_pos, direction);

            *h = Self::cast_ray(&mut r, &sphere_res); // calculate whether this ray hits any scene geometry
        });

        //Self::cast_rays(&mut self.sight_rays);
        hits
    }

    fn cast_ray(ray: &mut Ray, scene: &[SphereRenderQuery]) -> Option<HitInfo> {
        //TODO: if there are multiple spheres in the scene, calculate with SoA(structure of arrays) approach?

        // go through the scene, find the smallest distance
        let mut distance = NO_HIT;
        let mut nearest_entity: usize = usize::MAX;

        for (i, s) in scene.iter().enumerate() {
            let obj_distance = Self::ray_sphere_intersect(ray, s.transform.position, s.render.radius);
            if obj_distance < distance {
                distance = obj_distance;
                nearest_entity = i;
            };
        }

        if distance != NO_HIT {
            let position = ray.origin + (distance * ray.direction.normalize());
            let sphere_pos = scene[nearest_entity].transform.position;
            let normal = (position - sphere_pos).normalize();
            Some(HitInfo {
                object_id: 0,
                position,
                direction: ray.direction,
                normal,
            })
        } else {
            None
        }
    }

    #[inline]
    fn ray_sphere_intersect(ray: &mut Ray, sphere_pos: Vec3, sphere_radius: f32) -> f32 {
        // quadratic formula constants for line-sphere intersection

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
            let sqrt_discrim = discrim.sqrt();

            // now solve the quadratic, using a more stable computer friendly formula
            if unlikely(discrim == 0.0) {
                d0 = -0.5 * b / a;
                d1 = d0;
            } else {
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
