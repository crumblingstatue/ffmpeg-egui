use std::{
    ffi::CString,
    os::raw::{c_char, c_double, c_int},
};

/// # Safety
/// NAME must be null terminated
pub unsafe trait Property {
    type Type<'a>: PropertyType;
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

pub trait PropertyType {
    type CType: PropertyTypeRaw;
    fn from_c(src: Self::CType) -> Self;
    fn with_c<F>(self, f: F)
    where
        F: FnOnce(Self::CType);
}

impl PropertyType for f64 {
    type CType = c_double;

    fn with_c<F>(self, f: F)
    where
        F: FnOnce(Self::CType),
    {
        f(self)
    }

    fn from_c(src: Self::CType) -> Self {
        src
    }
}

impl PropertyType for i64 {
    type CType = i64;

    fn with_c<F>(self, f: F)
    where
        F: FnOnce(Self::CType),
    {
        f(self)
    }

    fn from_c(src: Self::CType) -> Self {
        src
    }
}

impl PropertyType for &str {
    type CType = *mut c_char;

    fn with_c<F>(self, f: F)
    where
        F: FnOnce(Self::CType),
    {
        let mut c_string = CString::new(self).unwrap().into_bytes_with_nul();
        f(c_string.as_mut_ptr() as *mut c_char)
    }

    fn from_c(_src: Self::CType) -> Self {
        todo!()
    }
}

impl PropertyType for bool {
    type CType = c_int;

    fn with_c<F>(self, f: F)
    where
        F: FnOnce(Self::CType),
    {
        f(match self {
            true => 1,
            false => 0,
        })
    }

    fn from_c(src: Self::CType) -> Self {
        match src {
            1 => true,
            0 => false,
            _ => panic!("Invalid value converting mpv property to bool: {}", src),
        }
    }
}

/// # Safety
/// This property must be writable
pub unsafe trait PropertyWrite: Property {}
