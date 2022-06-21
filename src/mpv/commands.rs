use std::ffi::CString;

use super::command::Command;

pub struct LoadFile<'a> {
    pub path: &'a str,
}

unsafe impl<'a> Command for LoadFile<'a> {
    const NAME: &'static str = "loadfile\0";
    const ARGS_COUNT: usize = 1;

    fn args(&self) -> [CString; Self::ARGS_COUNT] {
        [CString::new(self.path).unwrap()]
    }
}
