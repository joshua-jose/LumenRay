#![feature(core_intrinsics)]
#![feature(variant_count)]

pub mod engine;
pub mod renderer;
pub mod scene;
pub mod vk;

pub use glam::mat3;
pub use glam::mat4;
pub use glam::vec2;
pub use glam::vec3;
pub use glam::vec4;
pub use glam::Mat3;
pub use glam::Mat4;
pub use glam::Vec2;
pub use glam::Vec3;
pub use glam::Vec3Swizzles;
pub use glam::Vec4;

trait Reflectable {
    fn reflect(&self, normal: Self) -> Self;
}

impl Reflectable for Vec3 {
    fn reflect(&self, normal: Vec3) -> Vec3 { *self - normal * (2.0 * normal.dot(*self)) }
}
