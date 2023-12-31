use super::property::{Property, PropertyUnset, PropertyWrite, YesNo, YesNoAlways};

macro_rules! def_properties {
    ($($(#[$attr:meta])? $ident:ident, $str:literal, $ty:ty;)*) => {
        $(
            $(#[$attr])?
            pub enum $ident {}

            unsafe impl Property for $ident {
                type Type = $ty;
                const NAME: &'static str = $str;
            }
        )*
    };
}

def_properties! {
    /// The time position mpv is currently at
    TimePos, "time-pos\0", f64;
    Speed, "speed\0", f64;
    Volume, "volume\0", f64;
    Duration, "duration\0", f64;
    Pause, "pause\0", bool;
    AudioPitchCorrection, "audio-pitch-correction\0", bool;
    KeepOpen, "keep-open\0", YesNoAlways;
    KeepOpenPause, "keep-open-pause\0", YesNo;
    Width, "width\0", i64;
    Height, "height\0", i64;
    AbLoopA, "ab-loop-a\0", f64;
    AbLoopB, "ab-loop-b\0", f64;
    CropX, "video-params/crop-x\0", i64;
    CropY, "video-params/crop-y\0", i64;
    CropW, "video-params/crop-w\0", i64;
    CropH, "video-params/crop-h\0", i64;
}

unsafe impl PropertyWrite for TimePos {}
unsafe impl PropertyWrite for Speed {}
unsafe impl PropertyWrite for Volume {}
unsafe impl PropertyWrite for Pause {}
unsafe impl PropertyWrite for AudioPitchCorrection {}
unsafe impl PropertyWrite for KeepOpen {}
unsafe impl PropertyWrite for KeepOpenPause {}
unsafe impl PropertyWrite for AbLoopA {}
unsafe impl PropertyUnset for AbLoopA {
    type UnsetType = &'static str;

    const UNSET_VALUE: <Self as PropertyUnset>::UnsetType = "no";
}
unsafe impl PropertyWrite for AbLoopB {}
unsafe impl PropertyUnset for AbLoopB {
    type UnsetType = &'static str;

    const UNSET_VALUE: <Self as PropertyUnset>::UnsetType = "no";
}
