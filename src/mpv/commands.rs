use {
    super::command::Command,
    std::ffi::{CStr, CString},
};

pub struct LoadFile<'a> {
    pub path: &'a str,
}

unsafe impl Command for LoadFile<'_> {
    const NAME: &'static CStr = c"loadfile";
    const ARGS_COUNT: usize = 1;

    fn args(&self) -> [CString; Self::ARGS_COUNT] {
        [CString::new(self.path).unwrap()]
    }
}

pub struct FrameStep;

unsafe impl Command for FrameStep {
    const NAME: &'static CStr = c"frame-step";

    const ARGS_COUNT: usize = 0;

    fn args(&self) -> [CString; Self::ARGS_COUNT] {
        []
    }
}

pub struct FrameBackStep;

unsafe impl Command for FrameBackStep {
    const NAME: &'static CStr = c"frame-back-step";

    const ARGS_COUNT: usize = 0;

    fn args(&self) -> [CString; Self::ARGS_COUNT] {
        []
    }
}

pub enum PlaylistPlay {
    Index(u32),
    Current,
    None,
}

unsafe impl Command for PlaylistPlay {
    const NAME: &'static CStr = c"playlist-play-index";

    const ARGS_COUNT: usize = 1;

    fn args(&self) -> [CString; Self::ARGS_COUNT] {
        let buf: String;
        let s = match self {
            PlaylistPlay::Index(idx) => {
                buf = idx.to_string();
                &buf
            }
            PlaylistPlay::Current => "current",
            PlaylistPlay::None => "none",
        };
        [CString::new(s).unwrap()]
    }
}

pub struct SeekRelSeconds(pub f32);

unsafe impl Command for SeekRelSeconds {
    const NAME: &'static CStr = c"seek";
    const ARGS_COUNT: usize = 1;

    fn args(&self) -> [CString; Self::ARGS_COUNT] {
        [CString::new(self.0.to_string()).unwrap()]
    }
}
