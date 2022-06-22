use super::property::{Property, PropertyWrite, YesNo, YesNoAlways};

/// The time position mpv is currently at
pub enum TimePos {}

unsafe impl Property for TimePos {
    type Type<'a> = f64;
    const NAME: &'static str = "time-pos\0";
}

unsafe impl PropertyWrite for TimePos {}

pub enum Speed {}

unsafe impl Property for Speed {
    type Type<'a> = f64;
    const NAME: &'static str = "speed\0";
}

unsafe impl PropertyWrite for Speed {}

pub enum Volume {}

unsafe impl Property for Volume {
    type Type<'a> = f64;
    const NAME: &'static str = "volume\0";
}

unsafe impl PropertyWrite for Volume {}

pub enum Duration {}

unsafe impl Property for Duration {
    type Type<'a> = f64;
    const NAME: &'static str = "duration\0";
}

pub enum Pause {}

unsafe impl Property for Pause {
    type Type<'a> = bool;
    const NAME: &'static str = "pause\0";
}

unsafe impl PropertyWrite for Pause {}

pub enum AudioPitchCorrection {}

unsafe impl Property for AudioPitchCorrection {
    type Type<'a> = bool;

    const NAME: &'static str = "audio-pitch-correction\0";
}

unsafe impl PropertyWrite for AudioPitchCorrection {}

pub enum KeepOpen {}

unsafe impl Property for KeepOpen {
    type Type<'a> = YesNoAlways;

    const NAME: &'static str = "keep-open\0";
}

unsafe impl PropertyWrite for KeepOpen {}

pub enum KeepOpenPause {}

unsafe impl Property for KeepOpenPause {
    type Type<'a> = YesNo;

    const NAME: &'static str = "keep-open-pause\0";
}

unsafe impl PropertyWrite for KeepOpenPause {}

pub enum Width {}

unsafe impl Property for Width {
    type Type<'a> = i64;

    const NAME: &'static str = "width\0";
}

pub enum Height {}

unsafe impl Property for Height {
    type Type<'a> = i64;

    const NAME: &'static str = "height\0";
}
