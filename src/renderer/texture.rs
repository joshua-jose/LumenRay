use crate::{renderer::srgb_to_linear, Vec3};

pub struct Texture {
    pub(super) width:  u32,
    pub(super) height: u32,
    pub(super) data:   Vec<f32>,
}

impl Texture {
    pub fn from_path(path: &str) -> Self {
        let raw_image = image::open(path).unwrap().into_rgb8();
        let (width, height) = (raw_image.width(), raw_image.height());
        let mut data = Vec::with_capacity((raw_image.width() * raw_image.height()) as usize);

        for pixel in raw_image.pixels() {
            let [r, g, b] = pixel.0;
            //data.extend_from_slice(&rgb!(r, g, b).to_array());
            data.push(srgb_to_linear(r as f32 / 255.0));
            data.push(srgb_to_linear(g as f32 / 255.0));
            data.push(srgb_to_linear(b as f32 / 255.0));
            data.push(1.0);
        }
        Self { data, width, height }
    }

    pub fn from_colour_srgb(col: Vec3) -> Self {
        let data = vec![col.x, col.y, col.z, 1.0];

        Self {
            data,
            width: 1,
            height: 1,
        }
    }
}
