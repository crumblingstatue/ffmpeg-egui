use std::marker::PhantomData;

/// Video size magnitude
pub type VideoMag = u16;

#[derive(Debug)]
pub struct VideoVector<Kind, Space> {
    pub x: VideoMag,
    pub y: VideoMag,
    kind: PhantomData<Kind>,
    space: PhantomData<Space>,
}

impl<Kind, Space> Clone for VideoVector<Kind, Space> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Kind, Space> Copy for VideoVector<Kind, Space> {}

/// Dimension (w, h)
#[derive(Debug)]
pub enum Dim {}
pub type VideoDim<Space> = VideoVector<Dim, Space>;

/// Position (x, y)
#[derive(Debug)]
pub enum Pos {}
pub type VideoPos<Space> = VideoVector<Pos, Space>;

/// Source coordinate space
#[derive(Debug)]
pub enum Src {}

/// Present coordinate space
#[derive(Debug)]
pub enum Present {}

impl<Kind, Space> VideoVector<Kind, Space> {
    pub fn new(x: VideoMag, y: VideoMag) -> Self {
        Self {
            x,
            y,
            kind: PhantomData,
            space: PhantomData,
        }
    }
}

impl<Kind> VideoVector<Kind, Present> {
    pub fn to_src(
        self,
        src: VideoVector<Dim, Src>,
        present: VideoVector<Dim, Present>,
    ) -> VideoVector<Kind, Src> {
        let w_ratio = src.x as f64 / present.x as f64;
        let h_ratio = src.y as f64 / present.y as f64;
        VideoVector {
            x: (self.x as f64 * w_ratio) as VideoMag,
            y: (self.y as f64 * h_ratio) as VideoMag,
            kind: PhantomData,
            space: PhantomData,
        }
    }
}

impl<Kind> VideoVector<Kind, Src> {
    pub fn to_present(
        self,
        src: VideoVector<Dim, Src>,
        present: VideoVector<Dim, Present>,
    ) -> Self {
        let w_ratio = present.x as f64 / src.x as f64;
        let h_ratio = present.y as f64 / src.y as f64;
        Self {
            x: (self.x as f64 * w_ratio) as VideoMag,
            y: (self.y as f64 * h_ratio) as VideoMag,
            kind: PhantomData,
            space: PhantomData,
        }
    }
    pub fn as_present(self) -> VideoVector<Kind, Present> {
        VideoVector {
            x: self.x,
            y: self.y,
            kind: PhantomData,
            space: PhantomData,
        }
    }
}

impl VideoVector<Pos, Src> {
    pub(crate) fn from_present(
        x: i32,
        y: i32,
        src: VideoVector<Dim, Src>,
        present: VideoVector<Dim, Present>,
    ) -> Self {
        VideoVector::<Pos, Present>::new(x as u16, y as u16).to_src(src, present)
    }
}

#[derive(Debug)]
pub struct VideoRect<Space> {
    pub pos: VideoPos<Space>,
    pub dim: VideoDim<Space>,
}

impl<Kind> VideoRect<Kind> {
    pub fn new(x: VideoMag, y: VideoMag, w: VideoMag, h: VideoMag) -> Self {
        Self {
            pos: VideoPos::new(x, y),
            dim: VideoDim::new(w, h),
        }
    }
}

impl<Space> VideoVector<Dim, Space> {
    /// The length of an RGBA buffer that can hold the data of a video of this dimension
    pub fn rgba_bytes_len(&self) -> usize {
        usize::from(self.x) * usize::from(self.y) * 4
    }
}
