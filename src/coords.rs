/// Video dimension
#[derive(Clone, Copy)]
pub struct VideoDim {
    pub width: u16,
    pub height: u16,
}

/// Video position
#[derive(Clone, Copy)]
pub struct VideoPos {
    pub x: u16,
    pub y: u16,
}

impl VideoPos {
    pub fn present_from_src(src: Self, src_dim: VideoDim, present_dim: VideoDim) -> Self {
        let (x, y) = translate_up(src.x.into(), src.y.into(), src_dim, present_dim);
        Self { x, y }
    }
}

pub type VideoRect = sfml::graphics::Rect<u16>;

impl VideoDim {
    /// The length of an RGBA buffer that can hold the data of a video of this dimension
    pub fn rgba_bytes_len(&self) -> usize {
        usize::from(self.width) * usize::from(self.height) * 4
    }
    pub fn present_from_src(src: Self, src_dim: VideoDim, present_dim: VideoDim) -> Self {
        let (w, h) = translate_up(src.width.into(), src.height.into(), src_dim, present_dim);
        Self {
            width: w,
            height: h,
        }
    }
}

/// window -> vid coords
fn translate_down(x: i32, y: i32, src_dim: VideoDim, present_dim: VideoDim) -> (u16, u16) {
    let w_ratio = src_dim.width as f64 / present_dim.width as f64;
    let h_ratio = src_dim.height as f64 / present_dim.height as f64;
    ((x as f64 * w_ratio) as u16, (y as f64 * h_ratio) as u16)
}

/// vid -> window coords
fn translate_up(x: i32, y: i32, src_dim: VideoDim, present_dim: VideoDim) -> (u16, u16) {
    let w_ratio = present_dim.width as f64 / src_dim.width as f64;
    let h_ratio = present_dim.height as f64 / src_dim.height as f64;
    ((x as f64 * w_ratio) as u16, (y as f64 * h_ratio) as u16)
}
impl VideoPos {
    pub(crate) fn from_mouse(x: i32, y: i32, src: VideoDim, present: VideoDim) -> Self {
        let (x, y) = translate_down(x, y, src, present);
        Self { x, y }
    }
}
