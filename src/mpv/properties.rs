use {
    super::property::{Property, PropertyUnset, PropertyWrite, YesNo, YesNoAlways},
    std::ffi::CStr,
};

macro_rules! def_properties {
    ($($(#[$attr:meta])? $ident:ident, $str:literal, $ty:ty;)*) => {
        $(
            $(#[$attr])?
            pub enum $ident {}

            unsafe impl Property for $ident {
                type Type = $ty;
                const NAME: &'static CStr = $str;
            }
        )*
    };
}

def_properties! {
    /// The time position mpv is currently at
    TimePos, c"time-pos", f64;
    Speed, c"speed", f64;
    Volume, c"volume", f64;
    Duration, c"duration", f64;
    Pause, c"pause", bool;
    AudioPitchCorrection, c"audio-pitch-correction", bool;
    KeepOpen, c"keep-open", YesNoAlways;
    KeepOpenPause, c"keep-open-pause", YesNo;
    Width, c"width", i64;
    Height, c"height", i64;
    AbLoopA, c"ab-loop-a", f64;
    AbLoopB, c"ab-loop-b", f64;
    CropX, c"video-params/crop-x", i64;
    CropY, c"video-params/crop-y", i64;
    CropW, c"video-params/crop-w", i64;
    CropH, c"video-params/crop-h", i64;
    Rotate, c"video-params/rotate", i64;
    AudioId, c"aid", i64;
    SubId, c"sid", i64;
}

unsafe impl PropertyWrite for TimePos {}
unsafe impl PropertyWrite for Speed {}
unsafe impl PropertyWrite for Volume {}
unsafe impl PropertyWrite for Pause {}
unsafe impl PropertyWrite for AudioPitchCorrection {}
unsafe impl PropertyWrite for KeepOpen {}
unsafe impl PropertyWrite for KeepOpenPause {}
unsafe impl PropertyWrite for AbLoopA {}
unsafe impl PropertyWrite for AudioId {}
unsafe impl PropertyWrite for SubId {}
unsafe impl PropertyUnset for AbLoopA {
    type UnsetType = &'static str;

    const UNSET_VALUE: <Self as PropertyUnset>::UnsetType = "no";
}
unsafe impl PropertyWrite for AbLoopB {}
unsafe impl PropertyUnset for AbLoopB {
    type UnsetType = &'static str;

    const UNSET_VALUE: <Self as PropertyUnset>::UnsetType = "no";
}
