use super::SphereRenderQuery;
use crate::Vec3;

pub struct RenderScene<'a> {
    pub spheres:   Vec<SphereRenderQuery<'a>>,
    pub light_pos: Vec3,
}
