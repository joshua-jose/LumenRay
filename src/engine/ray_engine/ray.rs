use crate::engine::Vec3;

#[derive(Debug, Default, Clone, Copy)]
pub struct Ray {
    pub position:  Vec3,
    pub direction: Vec3,
    pub hit:       u32,
}

impl Ray {
    #[inline]
    pub const fn new(position: Vec3, direction: Vec3) -> Self {
        Self {
            position,
            direction,
            hit: u32::MAX,
        }
    }

    #[inline]
    pub fn march(&mut self, distance: f32) { self.position += self.direction * distance; }
}
