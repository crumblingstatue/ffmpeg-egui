use super::property::Property;

/// The time position mpv is currently at
pub enum TimePos {}

unsafe impl Property for TimePos {
    type Type = f64;
    const NAME: &'static str = "time-pos\0";
}

pub enum Duration {}

unsafe impl Property for Duration {
    type Type = f64;
    const NAME: &'static str = "duration\0";
}
