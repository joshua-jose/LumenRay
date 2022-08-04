use super::Ray;
use crate::engine::{vec3, Vec3};
use rayon::prelude::*;

//TODO: move to own file
struct HitInfo {
    object_id: usize,
    position:  Vec3,
    direction: Vec3,
    normal:    Vec3,
}

pub struct CPURayEngine {
    //scene:       Scene, // or take the hecs::world directly if it's more performant (but less desirable)
    entity_view: u32, // replace with hecs::PreparedView or LumenRay wrapper equiv
    // internal
    sight_rays:  Vec<Ray>, // keep a vec around, because we will likely generate sight rays every frame
}

// on draw, create direction vectors from transform

impl CPURayEngine {
    pub fn new(vecsize: usize) -> Self {
        Self {
            entity_view: 0,
            sight_rays:  vec![Ray::default(); vecsize],
        }
    }

    pub fn cast_sight_rays(&mut self /* , camera: &Camera*/, width: usize, height: usize) -> &Vec<Ray> {
        // generate rays based on camera info
        // ....
        // let origin = camera.position;
        // let width, height = camera.viewport_width, camera.viewport_height
        let now = std::time::Instant::now();

        if self.sight_rays.len() != width * height {
            self.sight_rays.resize(width * height, Ray::default());
        }

        let camera_pos = vec3(0.0, 0.0, 0.0);

        self.sight_rays.par_iter_mut().enumerate().for_each(|(i, r)| {
            // generate direction vectors from screen space UV coords
            let x = i % width;
            let y = i / width;

            let u = 2.0 * (x as f32 - (0.5 * (width as f32))) / height as f32; // divide u by height to account for aspect ratio
            let v = 2.0 * (y as f32 - (0.5 * (height as f32))) / height as f32;
            let direction = vec3(u, v, 1.0);

            *r = Ray::new(camera_pos, direction);

            Self::cast_ray(r); // calculate whether this ray hits any scene geometry
        });

        //Self::cast_rays(&mut self.sight_rays);
        // TODO: return hitinfo rather than ray?
        println!("Ray casting time: {:.2?}", now.elapsed());
        &self.sight_rays
    }

    fn cast_ray(ray: &mut Ray) {
        //TODO: if there are multiple spheres in the scene, calculate with SoA(structure of arrays) approach?
        let sphere_pos = vec3(0.0, 0.0, 5.0);
        let sphere_radius: f32 = 1.0;

        // quadratic formula constants for line-sphere intersection
        let a = ray.direction.length_squared();
        let b = 2.0 * (ray.direction.dot(ray.origin - sphere_pos));
        let c = (ray.origin - sphere_pos).length_squared() - sphere_radius.powi(2);

        let discrim = b.powi(2) - (4.0 * a * c);
        if discrim >= 0.0 {
            ray.hit = 0;
        }
    }
}
