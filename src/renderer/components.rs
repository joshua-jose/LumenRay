// put in SphereRender, PlaneRender, BoxRender, MeshRender etc. components here...

use crate::Vec3;

pub struct TransformComponent {
    pub position: Vec3,
}

impl TransformComponent {
    pub fn with_pos(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: Vec3 { x, y, z },
        }
    }
}
pub struct SphereRenderComponent {
    pub radius: f32,
}
