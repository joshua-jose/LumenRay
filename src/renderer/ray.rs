use crate::Vec3;

#[derive(Debug, Default, Clone, Copy)]
pub struct Ray {
    pub origin:    Vec3,
    pub direction: Vec3,
}

impl Ray {
    #[allow(dead_code)]
    #[inline]
    pub const fn new(origin: Vec3, direction: Vec3) -> Self { Self { origin, direction } }

    #[allow(dead_code)]
    #[inline]
    pub fn march(&mut self, distance: f32) {
        // FIXME: If we do end up doing some kind of marching, we would want to make this self.position, and make origin immutable
        // for now, we don't need to waste the memory on another Vec3
        self.origin += self.direction * distance;
    }
}
