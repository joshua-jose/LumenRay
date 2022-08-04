use super::Ray;
use crate::engine::{vec3, Vec3};
use rayon::prelude::*;

//TODO: move to own file
pub struct HitInfo {
    pub object_id: u32,
    pub position:  Vec3,
    pub direction: Vec3,
    pub normal:    Vec3,
}

pub struct CPURayEngine {
    //scene:       Scene, // or take the hecs::world directly if it's more performant (but less desirable)
    //entity_view: u32, // replace with hecs::PreparedView or LumenRay wrapper equiv
}

// on draw, create direction vectors from transform

impl CPURayEngine {
    pub fn new() -> Self { Self {} }

    #[allow(clippy::uninit_vec)]
    pub fn cast_sight_rays(&mut self /* , camera: &Camera*/, width: usize, height: usize) -> Vec<Option<HitInfo>> {
        // generate rays based on camera info
        // ....
        // let origin = camera.position;
        // let width, height = camera.viewport_width, camera.viewport_height
        let now = std::time::Instant::now();

        let mut sight_rays = Vec::with_capacity(width * height);
        unsafe {
            // we will not read from any of these locations until we have written them! This is safe, and so much faster.
            sight_rays.set_len(width * height);
        }

        let camera_pos = vec3(0.0, 0.0, 0.0);

        // use par_iter_mut to calculate across all cores
        sight_rays.par_iter_mut().enumerate().for_each(|(i, h)| {
            // generate direction vectors from screen space UV coords
            let x = i % width;
            let y = i / width;

            let u = 2.0 * (x as f32 - (0.5 * (width as f32))) / height as f32; // divide u by height to account for aspect ratio
            let v = 2.0 * (y as f32 - (0.5 * (height as f32))) / height as f32;
            let direction = vec3(u, v, 1.0);

            let mut r = Ray::new(camera_pos, direction);

            *h = Self::cast_ray(&mut r); // calculate whether this ray hits any scene geometry
        });

        //Self::cast_rays(&mut self.sight_rays);
        println!("Ray casting time: {:.2?}", now.elapsed());
        sight_rays
    }

    fn cast_ray(ray: &mut Ray) -> Option<HitInfo> {
        //TODO: if there are multiple spheres in the scene, calculate with SoA(structure of arrays) approach?
        let sphere_pos = vec3(0.0, 0.0, 5.0);
        let sphere_radius: f32 = 1.0;

        // TODO: split out ray-sphere intersection into function, then test analytical vs geometric approach
        // TODO: calculate and return position and normals, then use for colour
        //TODO: implement multiple objects, find min distance

        let distance = Self::ray_sphere_intersect(ray, sphere_pos, sphere_radius);

        if distance >= 0.0 {
            let position = ray.origin + (distance * ray.direction.normalize());
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
        let a = ray.direction.length_squared();
        let b = 2.0 * (ray.direction.dot(ray.origin - sphere_pos));
        let c = (ray.origin - sphere_pos).length_squared() - sphere_radius.powi(2);

        // distance of two intersection points
        let mut d0: f32 = -f32::MAX;
        let mut d1: f32;

        // b^2 - 4ac
        let discrim = b.powi(2) - (4.0 * a * c);

        if discrim >= 0.0 {
            let sqrt_discrim = discrim.sqrt();

            // now solve the quadratic, using a more stable computer friendly formula
            if std::intrinsics::unlikely(discrim == 0.0) {
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
            if d0 < 0.0 {
                d0 = d1;
                if d0 < 0.0 {
                    d0 = -f32::MAX;
                }
            }
        }

        d0
    }
}
