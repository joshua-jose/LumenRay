use super::Ray;
use crate::engine::{vec3, Vec3};
use rayon::prelude::*;

//TODO: move to own file
struct HitInfo {
    object_id: usize,
    position:  Vec3,
}

pub const MAX_MARCH_DISTANCE: f32 = 50.0;
pub const SMALL_DISTANCE: f32 = 0.001;

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

        // generate rays
        //TODO: try gen parallel
        /*
        for y in 0..height {
            for x in 0..width {
                let u = 2.0 * (x as f32 - (0.5 * (width as f32))) / height as f32; // divide u by height to account for aspect ratio
                let v = 2.0 * (y as f32 - (0.5 * (height as f32))) / height as f32;

                let i = x + (y * width);
                self.sight_rays[i] = Ray::new(vec3(0.0, 0.0, 0.0), vec3(u, v, 1.0).normalize());
            }
        }
         */
        self.sight_rays.par_iter_mut().enumerate().for_each(|(i, r)| {
            let x = i % width;
            let y = i / width;

            let u = 2.0 * (x as f32 - (0.5 * (width as f32))) / height as f32; // divide u by height to account for aspect ratio
            let v = 2.0 * (y as f32 - (0.5 * (height as f32))) / height as f32;

            *r = Ray::new(vec3(0.0, 0.0, 0.0), vec3(u, v, 1.0).normalize());
        });

        Self::cast_rays(&mut self.sight_rays);
        // TODO: return hitinfo rather than ray?
        println!("Ray casting time: {:.2?}", now.elapsed());
        &self.sight_rays
    }

    // internal ray casting function
    fn cast_rays(rays: &mut [Ray]) {
        let sphere_pos = vec3(0.0, 0.0, 5.0);
        let sphere_radius = 1.0;

        rays.par_iter_mut().for_each(|ray| {
            //TODO: try actual intersection func
            let mut total_distance = 0.0;

            while total_distance < MAX_MARCH_DISTANCE {
                //let distance = spheres.min_by_key(|s| sphere.sdf());
                let distance = (ray.position - sphere_pos).length() - sphere_radius;

                if distance < SMALL_DISTANCE {
                    ray.hit = 0;
                    break;
                }

                total_distance += distance;
                ray.march(distance);
            }
        });
    }
}
