// put in SphereRender, PlaneRender, BoxRender, MeshRender etc. components here...

use crate::Vec3;

use super::SOFT_BLUE;

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

pub struct MaterialComponent {
    pub colour:       Vec3,
    pub ambient:      f32,
    pub diffuse:      f32, // aka albedo
    pub specular:     f32,
    pub shininess:    f32, // aka gloss
    pub reflectivity: f32,
    pub emissive:     f32,
}

impl MaterialComponent {
    pub const fn basic() -> Self {
        Self {
            colour:       SOFT_BLUE,
            ambient:      0.25,
            diffuse:      1.0,
            specular:     0.0,
            shininess:    4.0,
            reflectivity: 0.0,
            emissive:     0.0,
        }
    }
}

impl Default for MaterialComponent {
    fn default() -> Self { Self::basic() }
}
