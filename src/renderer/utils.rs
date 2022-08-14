use crate::Vec3;

#[macro_export]
macro_rules! rgb {
    [$r:expr, $g:expr, $b:expr] => {
        Vec3::new(
             ($r as f32 / 255.0) * ($r as f32 / 255.0),
             ($g as f32 / 255.0) * ($g as f32 / 255.0),
             ($b as f32 / 255.0) * ($b as f32 / 255.0),
        )
    };
}

pub const WHITE: Vec3 = rgb![255, 255, 255];
pub const SOFT_RED: Vec3 = rgb![214, 81, 81];
pub const SOFT_GREEN: Vec3 = rgb![81, 214, 81];
pub const SOFT_BLUE: Vec3 = rgb![81, 81, 214];
pub const SOFT_GRAY: Vec3 = rgb![214, 214, 214];
pub const SOFT_YELLOW: Vec3 = rgb![230, 230, 127];
