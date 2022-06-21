use std::ffi::CString;

/// # Safety
/// NAME must be null terminated
pub unsafe trait Command {
    const NAME: &'static str;
    const ARGS_COUNT: usize;
    fn args(&self) -> [CString; Self::ARGS_COUNT];
}
