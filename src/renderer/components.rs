// put in SphereRender, PlaneRender, BoxRender, MeshRender etc. components here...

use crate::{vec3, Mat3, Vec3};

use super::SOFT_BLUE;

pub struct TransformComponent {
    pub position: Vec3,
    // rotation
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

pub struct PlaneRenderComponent {
    pub normal: Vec3,
}

pub struct MaterialComponent {
    pub colour:       Vec3, // TODO: Replace with texture
    pub ambient:      f32,
    pub diffuse:      f32, // aka albedo
    pub specular:     f32,
    pub shininess:    f32, // aka gloss
    pub reflectivity: f32,
    pub emissive:     f32,
    // TODO: specular tint?
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

pub struct CameraComponent {
    pub pitch: f32,
    pub yaw:   f32,
    pub fov:   f32,
    // exposure
}

impl CameraComponent {
    pub fn get_rot_mat(&self) -> Mat3 {
        let (sx, cx) = self.yaw.sin_cos();
        let (sy, cy) = self.pitch.sin_cos();

        let rot_pitch = Mat3::from_cols(vec3(1.0, 0.0, 0.0), vec3(0.0, cy, sy), vec3(0.0, -sy, cy));
        let rot_yaw = Mat3::from_cols(vec3(cx, 0.0, -sx), vec3(0.0, 1.0, 0.0), vec3(sx, 0.0, cx));
        rot_yaw * rot_pitch
    }
}

pub struct PointLightComponent {
    pub intensity: f32, // shadow softness?
}

pub struct SkyBoxComponent {}
