use std::ffi::CStr;

pub use super::property::Flag;
use super::property::{Property, PropertyWrite};

/// The time position mpv is currently at
pub enum TimePos {}

unsafe impl Property for TimePos {
    type Type = f64;
    const NAME: &'static str = "time-pos\0";
}

unsafe impl PropertyWrite for TimePos {}

pub enum Speed {}

unsafe impl Property for Speed {
    type Type = f64;
    const NAME: &'static str = "speed\0";
}

unsafe impl PropertyWrite for Speed {}

pub enum Volume {}

unsafe impl Property for Volume {
    type Type = f64;
    const NAME: &'static str = "volume\0";
}

unsafe impl PropertyWrite for Volume {}

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

pub enum AudioPitchCorrection {}

unsafe impl super::property::Option for AudioPitchCorrection {
    type Type<'a> = Flag;

    const NAME: &'static str = "audio-pitch-correction\0";
}

pub enum KeepOpen {}

unsafe impl super::property::Option for KeepOpen {
    type Type<'a> = &'a CStr;

    const NAME: &'static str = "keep-open\0";
}

pub enum KeepOpenPause {}

unsafe impl super::property::Option for KeepOpenPause {
    type Type<'a> = &'a CStr;

    const NAME: &'static str = "keep-open-pause\0";
}

pub enum Width {}

unsafe impl Property for Width {
    type Type = i64;

    const NAME: &'static str = "width\0";
}

pub enum Height {}

unsafe impl Property for Height {
    type Type = i64;

    const NAME: &'static str = "height\0";
}
