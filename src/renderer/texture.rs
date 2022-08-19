pub struct Texture {
    pub width:  f32,
    pub height: f32,

    data: Vec<f32>, // Load data when creating object in RAII manner
                    // When texture is first used in a render pass, load it onto GPU if not already there
}
