use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int, c_void},
};

use libmpv_sys as ffi;

use crate::video_pix_size;

pub struct Mpv {
    mpv_handle: *mut ffi::mpv_handle,
    render_ctx: *mut ffi::mpv_render_context,
    pix_buf: Vec<u8>,
}

impl Mpv {
    pub fn new() -> anyhow::Result<Self> {
        let mpv_handle = unsafe { ffi::mpv_create() };
        if mpv_handle.is_null() {
            panic!("Failed to create mpv instance");
        }
        let render_ctx = unsafe {
            if ffi::mpv_initialize(mpv_handle) < 0 {
                panic!("Failed to initialize mpv");
            }
            let mut ctx = std::ptr::null_mut();
            let sw_render_param: &[u8; 3] = ffi::MPV_RENDER_API_TYPE_SW;
            let mut ctrl_param: std::os::raw::c_int = 1;
            let mut params = [
                ffi::mpv_render_param {
                    type_: ffi::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
                    data: sw_render_param.as_ptr() as _,
                },
                ffi::mpv_render_param {
                    type_: ffi::mpv_render_param_type_MPV_RENDER_PARAM_ADVANCED_CONTROL,
                    data: (&mut ctrl_param) as *mut _ as _,
                },
                std::mem::zeroed(),
            ];
            let ret_val = ffi::mpv_render_context_create(&mut ctx, mpv_handle, params.as_mut_ptr());
            if ret_val < 0 {
                panic!(
                    "Failed to init render context: {}",
                    ffi::mpv_error_str(ret_val)
                );
            }
            ctx
        };
        assert!(!render_ctx.is_null());
        Ok(Self {
            mpv_handle,
            render_ctx,
            pix_buf: Vec::new(),
        })
    }
    pub fn command_async(&mut self, command: Command) {
        command.with_as_ptr(|slice| unsafe {
            ffi::mpv_command_async(self.mpv_handle, 0, slice.as_mut_ptr());
        });
    }
    pub fn get_frame_as_pixels(&mut self, video_w: u16, video_h: u16) -> &[u8] {
        let pix_size = video_pix_size(video_w, video_h);
        if self.pix_buf.len() != pix_size {
            self.pix_buf.resize(pix_size, 0);
        }
        unsafe {
            let mut size: [c_int; 2] = [c_int::from(video_w), c_int::from(video_h)];
            let mut format = *b"rgb0\0";
            let mut stride: usize = video_w as usize * 4;
            let mut params = [
                ffi::mpv_render_param {
                    type_: ffi::mpv_render_param_type_MPV_RENDER_PARAM_SW_SIZE,
                    data: size.as_mut_ptr() as _,
                },
                ffi::mpv_render_param {
                    type_: ffi::mpv_render_param_type_MPV_RENDER_PARAM_SW_FORMAT,
                    data: format.as_mut_ptr() as _,
                },
                ffi::mpv_render_param {
                    type_: ffi::mpv_render_param_type_MPV_RENDER_PARAM_SW_STRIDE,
                    data: (&mut stride) as *mut _ as _,
                },
                ffi::mpv_render_param {
                    type_: ffi::mpv_render_param_type_MPV_RENDER_PARAM_SW_POINTER,
                    data: self.pix_buf.as_mut_ptr() as _,
                },
                std::mem::zeroed(),
            ];
            let result = ffi::mpv_render_context_render(self.render_ctx, params.as_mut_ptr());
            for [.., a] in self.pix_buf.array_chunks_mut::<4>() {
                *a = 255;
            }
            if result < 0 {
                eprintln!("Render error: {}", ffi::mpv_error_str(result));
            }
            &self.pix_buf
        }
    }

    pub fn get_property_string(&self, name: &str) -> Option<String> {
        let name_c_string = CString::new(name).unwrap();
        let c_str =
            unsafe { ffi::mpv_get_property_string(self.mpv_handle, name_c_string.as_ptr() as _) };
        if c_str.is_null() {
            None
        } else {
            unsafe {
                let my_c_str = CStr::from_ptr(c_str);
                let string = my_c_str.to_str().unwrap().to_string();
                ffi::mpv_free(c_str as *mut c_void);
                Some(string)
            }
        }
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        unsafe {
            ffi::mpv_render_context_free(self.render_ctx);
            ffi::mpv_destroy(self.mpv_handle);
        }
    }
}

pub enum Command<'a> {
    LoadFile { path: &'a str },
}

impl<'a> Command<'a> {
    fn with_as_ptr<F>(&self, f: F)
    where
        F: FnOnce(&mut [*const c_char]),
    {
        match *self {
            Command::LoadFile { path } => {
                let path = CString::new(path).unwrap();
                f(&mut [
                    b"loadfile\0".as_ptr() as *const c_char,
                    path.as_ptr() as *const c_char,
                    std::ptr::null(),
                ][..])
            }
        };
    }
}
