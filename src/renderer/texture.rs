use crate::{renderer::srgb_to_linear, rgb, Vec3};

pub struct Texture {
    // Unique ID per texture, a texture can be shared so multiple entities can point to same texture ID.
    // When getting a texture ID, make it only gettable from the renderer, given a filepath. this way we can repeat IDs
    // for the same filepath. All entities can be updated if a texture gets updated from the renderer.
    // `renderer.get_texture(filepath)`, `renderer.update_texture(filepath)`
    //id: u32,
    pub(super) width:  u32,
    pub(super) height: u32,
    pub(super) uscale: f32,
    pub(super) vscale: f32,
    pub(super) data:   Vec<Vec3>,
}

impl Texture {
    pub fn from_path(path: &str, uscale: f32, vscale: f32) -> Self {
        let raw_image = image::open(path).unwrap().into_rgb8();
        let (width, height) = (raw_image.width(), raw_image.height());
        let mut data = Vec::with_capacity((raw_image.width() * raw_image.height()) as usize);

        for pixel in raw_image.pixels() {
            let [r, g, b] = pixel.0;
            data.push(rgb!(r, g, b))
        }
        Self {
            data,
            width,
            height,
            uscale,
            vscale,
        }
    }

    pub fn from_colour_srgb(col: Vec3) -> Self {
        let data = vec![col];

        Self {
            data,
            width: 1,
            height: 1,
            uscale: 1.0,
            vscale: 1.0,
        }
    }

    pub fn sample(&self, u: f32, v: f32) -> Vec3 {
        // nearest neighbour sampled
        let width = self.width as f32;
        let height = self.height as f32;
        let x = ((u * self.uscale * width) % width).abs().floor() as u32;
        let y = ((v * self.vscale * height) % height).abs().floor() as u32;
        let i = x + (y * self.width);

        self.data[i as usize]
    }
}
