use {
    crate::coords::VideoDim,
    egui_sfml::sfml::{cpp::FBox, graphics::Texture},
};

pub struct Present {
    pub dim: VideoDim<crate::coords::Present>,
    pub texture: FBox<Texture>,
}

impl Present {
    pub fn new(dim: VideoDim<crate::coords::Present>) -> Option<Self> {
        if dim.x == 0 || dim.y == 0 {
            eprintln!("Present::new: Zero dimension(s) passed");
            return None;
        }
        let mut texture = Texture::new().unwrap();
        if texture
            .create(dim.x.try_into().unwrap(), dim.y.try_into().unwrap())
            .is_err()
        {
            eprintln!("Failed to create texture");
        }
        Some(Present { dim, texture })
    }
}
