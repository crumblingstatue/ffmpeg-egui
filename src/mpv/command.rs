use std::ffi::{CStr, CString};

/// A command to make mpv perform some kind of operation
///
/// # Safety
///
/// Must make sure `ARGS_COUNT` is correct
pub unsafe trait Command {
    const NAME: &'static CStr;
    const ARGS_COUNT: usize;
    fn args(&self) -> [CString; Self::ARGS_COUNT];
}
