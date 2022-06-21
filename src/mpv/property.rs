use std::os::raw::c_int;

/// # Safety
/// NAME must be null terminated
pub unsafe trait Property {
    type Type: PropertyType;
    const NAME: &'static str;
}

/// # Safety
/// NAME must be null terminated
pub unsafe trait Option {
    type Type: PropertyType;
    const NAME: &'static str;
}

/// # Safety
/// FORMAT must be the correct format for this type
pub unsafe trait PropertyType {
    const FORMAT: libmpv_sys::mpv_format;
}

unsafe impl PropertyType for f64 {
    const FORMAT: libmpv_sys::mpv_format = libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE;
}

#[repr(transparent)]
#[derive(PartialEq, Eq)]
pub struct Flag(c_int);

unsafe impl PropertyType for Flag {
    const FORMAT: libmpv_sys::mpv_format = libmpv_sys::mpv_format_MPV_FORMAT_FLAG;
}

impl Flag {
    pub const NO: Self = Self(0);
    pub const YES: Self = Self(1);
}

/// # Safety
/// This property must be writable
pub unsafe trait PropertyWrite: Property {}
