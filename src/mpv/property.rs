use std::os::raw::{c_char, c_double, c_int};

/// # Safety
/// NAME must be null terminated
pub unsafe trait Property {
    type Type<'a>: PropertyTypeRaw;
    const NAME: &'static str;
}

/// # Safety
/// FORMAT must be the correct format for this type
pub unsafe trait PropertyTypeRaw {
    const FORMAT: libmpv_sys::mpv_format;
}

unsafe impl PropertyTypeRaw for c_double {
    const FORMAT: libmpv_sys::mpv_format = libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE;
}

unsafe impl PropertyTypeRaw for i64 {
    const FORMAT: libmpv_sys::mpv_format = libmpv_sys::mpv_format_MPV_FORMAT_INT64;
}

unsafe impl PropertyTypeRaw for *mut c_char {
    const FORMAT: libmpv_sys::mpv_format = libmpv_sys::mpv_format_MPV_FORMAT_STRING;
}

unsafe impl PropertyTypeRaw for c_int {
    const FORMAT: libmpv_sys::mpv_format = libmpv_sys::mpv_format_MPV_FORMAT_FLAG;
}

/// # Safety
/// This property must be writable
pub unsafe trait PropertyWrite: Property {}
