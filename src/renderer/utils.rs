pub fn srgb_to_linear(x: f32) -> f32 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

#[macro_export]
macro_rules! rgb {
    [$r:expr, $g:expr, $b:expr] => {
        Vec3::new(
             srgb_to_linear($r as f32 / 255.0),
             srgb_to_linear($g as f32 / 255.0),
             srgb_to_linear($b as f32 / 255.0),
        )
    };
}
#[macro_export]
macro_rules! white {
    () => {
        rgb![255, 255, 255]
    };
}
#[macro_export]
macro_rules! soft_red {
    () => {
        rgb![235, 40, 40]
    };
}
#[macro_export]
macro_rules! soft_green {
    () => {
        rgb![40, 235, 40]
    };
}
#[macro_export]
macro_rules! soft_blue {
    () => {
        rgb![40, 40, 235]
    };
}
#[macro_export]
macro_rules! soft_gray {
    () => {
        rgb![235, 235, 235]
    };
}
#[macro_export]
macro_rules! soft_yellow {
    () => {
        rgb![230, 230, 127]
    };
}
