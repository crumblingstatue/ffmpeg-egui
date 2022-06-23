use std::marker::PhantomData;

/// Video size magnitude
pub type VideoMag = u16;

pub struct VideoVector<Kind> {
    pub x: VideoMag,
    pub y: VideoMag,
    kind: PhantomData<Kind>,
}

impl<Kind> Clone for VideoVector<Kind> {
    fn clone(&self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            kind: PhantomData,
        }
    }
}
impl<Kind> Copy for VideoVector<Kind> {}

/// Dimension (w, h)
pub enum Dim {}
pub type VideoDim = VideoVector<Dim>;

/// Position (x, y)
pub enum Pos {}
pub type VideoPos = VideoVector<Pos>;

impl<Kind> VideoVector<Kind> {
    pub fn new(x: VideoMag, y: VideoMag) -> Self {
        Self {
            x,
            y,
            kind: PhantomData,
        }
    }
    pub fn to_src(self, src: VideoVector<Dim>, present: VideoVector<Dim>) -> Self {
        let w_ratio = src.x as f64 / present.x as f64;
        let h_ratio = src.y as f64 / present.y as f64;
        Self {
            x: (self.x as f64 * w_ratio) as VideoMag,
            y: (self.y as f64 * h_ratio) as VideoMag,
            kind: PhantomData,
        }
    }
    pub fn to_present(self, src: VideoVector<Dim>, present: VideoVector<Dim>) -> Self {
        let w_ratio = present.x as f64 / src.x as f64;
        let h_ratio = present.y as f64 / src.y as f64;
        Self {
            x: (self.x as f64 * w_ratio) as VideoMag,
            y: (self.y as f64 * h_ratio) as VideoMag,
            kind: PhantomData,
        }
    }
}

impl VideoVector<Pos> {
    pub(crate) fn from_present(
        x: i32,
        y: i32,
        src: VideoVector<Dim>,
        present: VideoVector<Dim>,
    ) -> Self {
        Self::new(x as u16, y as u16).to_src(src, present)
    }
}

pub struct VideoRect {
    pub pos: VideoPos,
    pub dim: VideoDim,
}

impl VideoRect {
    pub fn new(x: VideoMag, y: VideoMag, w: VideoMag, h: VideoMag) -> Self {
        Self {
            pos: VideoPos::new(x, y),
            dim: VideoDim::new(w, h),
        }
    }
}

impl VideoVector<Dim> {
    /// The length of an RGBA buffer that can hold the data of a video of this dimension
    pub fn rgba_bytes_len(&self) -> usize {
        usize::from(self.x) * usize::from(self.y) * 4
    }
}
