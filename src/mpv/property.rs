/// # Safety
/// NAME must be null terminated
pub unsafe trait Property {
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

/// # Safety
/// This property must be writable
pub unsafe trait PropertyWrite: Property {}
