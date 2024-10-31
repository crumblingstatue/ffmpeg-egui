use {
    crate::coords::VideoDim,
    egui_sfml::sfml::{graphics::Texture, SfBox},
};

pub struct Present {
    pub dim: VideoDim<crate::coords::Present>,
    pub texture: SfBox<Texture>,
}

impl Present {
    pub fn new(dim: VideoDim<crate::coords::Present>) -> Self {
        let mut texture = Texture::new().unwrap();
        if texture
            .create(dim.x.try_into().unwrap(), dim.y.try_into().unwrap())
            .is_err()
        {
            eprintln!("Failed to create texture");
        }
        Present { dim, texture }
    }
}
