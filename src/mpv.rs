mod command;
pub mod commands;
pub mod properties;
mod property;

use std::{
    mem::MaybeUninit,
    os::raw::{c_int, c_void},
};

use libmpv_sys as ffi;

use crate::video_pix_size;

use self::{
    command::Command,
    property::{Property, PropertyType, PropertyTypeRaw, PropertyWrite},
};

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
    pub fn command_async<C: Command>(&mut self, command: C)
    where
        [(); C::ARGS_COUNT + 2]:,
    {
        let mut args_buf = [std::ptr::null(); C::ARGS_COUNT + 2];
        args_buf[0] = C::NAME.as_ptr() as *const i8;
        let args = command.args();
        for (i, arg) in args.iter().enumerate() {
            args_buf[i + 1] = arg.as_ptr();
        }
        *args_buf.last_mut().unwrap() = std::ptr::null();
        unsafe {
            ffi::mpv_command_async(self.mpv_handle, 0, args_buf.as_mut_ptr());
        }
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

    /// See [`properties`] for the properties you can use.
    pub fn get_property<P: Property>(&self) -> Option<P::Type<'_>> {
        let mut out: MaybeUninit<<P::Type<'_> as PropertyType>::CType> = MaybeUninit::uninit();
        unsafe {
            if ffi::mpv_get_property(
                self.mpv_handle,
                P::NAME.as_bytes().as_ptr() as _,
                <P::Type<'_> as PropertyType>::CType::FORMAT,
                out.as_mut_ptr() as _,
            ) < 0
            {
                None
            } else {
                let c_val = out.assume_init();
                Some(P::Type::from_c(c_val))
            }
        }
    }

    /// See [`properties`] for the properties you can use.
    pub fn set_property<P: PropertyWrite>(&self, value: P::Type<'_>) -> bool {
        let mut ret = false;
        value.with_c(|mut cvalue| unsafe {
            ret = ffi::mpv_set_property(
                self.mpv_handle,
                P::NAME.as_bytes().as_ptr() as _,
                <P::Type<'_> as PropertyType>::CType::FORMAT,
                (&mut cvalue) as *mut _ as *mut c_void,
            ) >= 0
        });
        ret
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
