use super::{QueryTrait, Scene};
use crate::{
    renderer::{MaterialComponent, PlaneRenderComponent, SphereRenderComponent, TransformComponent},
    vec3, Vec3,
};

#[derive(QueryTrait)]
pub struct SphereRenderQuery<'a> {
    pub transform: &'a TransformComponent,
    pub render:    &'a SphereRenderComponent,
    pub material:  &'a MaterialComponent,
}

#[derive(QueryTrait)]
pub struct PlaneRenderQuery<'a> {
    pub transform: &'a TransformComponent,
    pub render:    &'a PlaneRenderComponent,
    pub material:  &'a MaterialComponent,
}

pub struct RenderScene<'a> {
    pub spheres:   Vec<(u32, SphereRenderQuery<'a>)>,
    pub planes:    Vec<(u32, PlaneRenderQuery<'a>)>,
    pub light_pos: Vec3,
}

impl<'a> RenderScene<'a> {
    pub fn from_scene(scene: &'a mut Scene) -> Self {
        //let now = std::time::Instant::now();

        // As a lightweight wrapper, we return entity id's as u32s, rather than returning a whole Entity object, since it needs to be very light
        let sphere_res = scene.query_owned::<SphereRenderQuery>().into_iter();
        let spheres = sphere_res.map(|(e, s)| (e.id(), s)).collect::<Vec<_>>();

        let plane_res = scene.query_owned::<PlaneRenderQuery>().into_iter();
        let planes = plane_res.map(|(e, p)| (e.id(), p)).collect::<Vec<_>>();
        //println!("Query time: {:.2?}", now.elapsed());
        Self {
            spheres,
            planes,
            light_pos: vec3(0.0, 0.0, 0.0),
        }
    }

    pub fn get_sphere_by_id(&self, target_id: u32) -> Option<&SphereRenderQuery> {
        let idx = self.spheres.binary_search_by_key(&target_id, |&(id, _)| id).ok()?;
        Some(&self.spheres[idx].1)
    }

    pub fn get_plane_by_id(&self, target_id: u32) -> Option<&PlaneRenderQuery> {
        let idx = self.planes.binary_search_by_key(&target_id, |&(id, _)| id).ok()?;
        Some(&self.planes[idx].1)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ObjectType {
    None,
    Sphere,
    Plane,
    Box,
    Mesh,
}
