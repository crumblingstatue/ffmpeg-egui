use std::{ffi::CStr, os::raw::c_int};

/// # Safety
/// NAME must be null terminated
pub unsafe trait Property {
    type Type<'a>: PropertyType;
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

unsafe impl PropertyType for i64 {
    const FORMAT: libmpv_sys::mpv_format = libmpv_sys::mpv_format_MPV_FORMAT_INT64;
}

unsafe impl<'a> PropertyType for &'a CStr {
    const FORMAT: libmpv_sys::mpv_format = libmpv_sys::mpv_format_MPV_FORMAT_STRING;
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
