/// Video dimension (width, height)
#[derive(Clone, Copy)]
pub struct VideoDim {
    pub width: u16,
    pub height: u16,
}

pub fn video_mouse_pos(
    mouse_pos: sfml::system::Vector2<i32>,
    src_dim: VideoDim,
    present_dim: VideoDim,
) -> (i16, i16) {
    translate_down(mouse_pos.x, mouse_pos.y, src_dim, present_dim)
}

/// window -> vid coords
pub fn translate_down(x: i32, y: i32, src_dim: VideoDim, present_dim: VideoDim) -> (i16, i16) {
    let w_ratio = src_dim.width as f64 / present_dim.width as f64;
    let h_ratio = src_dim.height as f64 / present_dim.height as f64;
    ((x as f64 * w_ratio) as i16, (y as f64 * h_ratio) as i16)
}

/// vid -> window coords
pub fn translate_up(x: i32, y: i32, src_dim: VideoDim, present_dim: VideoDim) -> (i16, i16) {
    let w_ratio = present_dim.width as f64 / src_dim.width as f64;
    let h_ratio = present_dim.height as f64 / src_dim.height as f64;
    ((x as f64 * w_ratio) as i16, (y as f64 * h_ratio) as i16)
}
