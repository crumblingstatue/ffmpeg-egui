use {crate::coords::VideoPos, egui_sfml::sfml::system::Vector2f};

pub trait VideoPosSfExt {
    fn to_sf(&self) -> Vector2f;
}

impl<Space> VideoPosSfExt for VideoPos<Space> {
    fn to_sf(&self) -> Vector2f {
        Vector2f::new(self.x.into(), self.y.into())
    }
}
