use {
    crate::{coords::VideoPos, ui::EguiFriendlyColor},
    egui_sf2g::sf2g::{graphics::Color, system::Vector2f},
};

pub trait VideoPosSfExt {
    fn to_sf(&self) -> Vector2f;
    fn to_arr(&self) -> [f32; 2];
}

impl<Space> VideoPosSfExt for VideoPos<Space> {
    fn to_sf(&self) -> Vector2f {
        Vector2f::new(self.x.into(), self.y.into())
    }
    fn to_arr(&self) -> [f32; 2] {
        [self.x.into(), self.y.into()]
    }
}

pub trait EguiFriendlyColorExt {
    fn to_sfml(self) -> Color;
}

impl EguiFriendlyColorExt for EguiFriendlyColor {
    fn to_sfml(self) -> Color {
        let [r, g, b] = self;
        Color::rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }
}
