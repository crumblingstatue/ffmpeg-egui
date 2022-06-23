use sfml::{graphics::Texture, SfBox};

use crate::coords::VideoDim;

pub struct Present {
    pub dim: VideoDim<crate::coords::Present>,
    pub texture: SfBox<Texture>,
}

impl Present {
    pub fn new(dim: VideoDim<crate::coords::Present>) -> Self {
        let mut texture = Texture::new().unwrap();
        if !texture.create(dim.x.into(), dim.y.into()) {
            panic!("Failed to create texture");
        }
        Present { dim, texture }
    }
}
