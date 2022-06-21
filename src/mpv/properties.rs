pub use super::property::Flag;
use super::property::{Property, PropertyWrite};

/// The time position mpv is currently at
pub enum TimePos {}

unsafe impl Property for TimePos {
    type Type = f64;
    const NAME: &'static str = "time-pos\0";
}

unsafe impl PropertyWrite for TimePos {}

pub enum Duration {}

unsafe impl Property for Duration {
    type Type = f64;
    const NAME: &'static str = "duration\0";
}

pub enum Pause {}

unsafe impl Property for Pause {
    type Type = Flag;
    const NAME: &'static str = "pause\0";
}

unsafe impl PropertyWrite for Pause {}