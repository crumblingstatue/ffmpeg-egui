use crate::coords::{Src, VideoDim};

pub struct Info {
    pub dim: VideoDim<Src>,
    pub w_h_ratio: f64,
    pub duration: f64,
}
