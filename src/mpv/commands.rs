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

pub struct FrameStep;

unsafe impl Command for FrameStep {
    const NAME: &'static str = "frame-step\0";

    const ARGS_COUNT: usize = 0;

    fn args(&self) -> [CString; Self::ARGS_COUNT] {
        []
    }
}

pub struct FrameBackStep;

unsafe impl Command for FrameBackStep {
    const NAME: &'static str = "frame-back-step\0";

    const ARGS_COUNT: usize = 0;

    fn args(&self) -> [CString; Self::ARGS_COUNT] {
        []
    }
}
