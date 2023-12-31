use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_double, c_int},
};

/// # Safety
/// NAME must be null terminated
pub unsafe trait Property {
    type Type: PropertyType;
    const NAME: &'static CStr;
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

pub enum YesNo {
    Yes,
    No,
}

impl PropertyType for YesNo {
    type CType = *mut c_char;

    fn from_c(src: Self::CType) -> Self {
        let c_str = unsafe { CStr::from_ptr(src) };
        match c_str.to_str().unwrap() {
            "yes" => YesNo::Yes,
            "no" => YesNo::No,
            etc => panic!("Invalid yes/no option: {}", etc),
        }
    }

    fn with_c<F>(self, f: F)
    where
        F: FnOnce(Self::CType),
    {
        match self {
            YesNo::Yes => f(b"yes\0".as_ptr() as *mut c_char),
            YesNo::No => f(b"no\0".as_ptr() as *mut c_char),
        }
    }
}

pub enum YesNoAlways {
    Yes,
    No,
    Always,
}

impl PropertyType for YesNoAlways {
    type CType = *mut c_char;

    fn from_c(src: Self::CType) -> Self {
        let c_str = unsafe { CStr::from_ptr(src) };
        match c_str.to_str().unwrap() {
            "yes" => YesNoAlways::Yes,
            "no" => YesNoAlways::No,
            "always" => YesNoAlways::Always,
            etc => panic!("Invalid yes/no option: {}", etc),
        }
    }

    fn with_c<F>(self, f: F)
    where
        F: FnOnce(Self::CType),
    {
        match self {
            YesNoAlways::Yes => f(b"yes\0".as_ptr() as *mut c_char),
            YesNoAlways::No => f(b"no\0".as_ptr() as *mut c_char),
            YesNoAlways::Always => f(b"always\0".as_ptr() as *mut c_char),
        }
    }
}

/// # Safety
/// This property must be writable
pub unsafe trait PropertyWrite: Property {}

/// # Safety
/// Must specify the correct type and value to "unset" this property
pub unsafe trait PropertyUnset: Property {
    type UnsetType: PropertyType;
    const UNSET_VALUE: Self::UnsetType;
}
