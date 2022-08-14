#![feature(core_intrinsics)]

pub mod renderer;
pub mod scene;
pub mod vk;

pub use glam::vec3;
pub use glam::vec4;
pub use glam::Vec3;
pub use glam::Vec3Swizzles;
pub use glam::Vec4;

trait Reflectable {
    fn reflect(&self, normal: Self) -> Self;
}

impl Reflectable for Vec3 {
    fn reflect(&self, normal: Vec3) -> Vec3 { *self - normal * (2.0 * normal.dot(*self)) }
}
