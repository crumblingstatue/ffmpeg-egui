use {
    self::{
        command::Command,
        property::{Property, PropertyType, PropertyTypeRaw, PropertyUnset, PropertyWrite},
    },
    crate::coords::{Present, VideoDim},
    libmpv_sys::{self as ffi, mpv_load_config_file},
    std::{
        mem::MaybeUninit,
        os::raw::{c_int, c_void},
    },
};

mod command;
pub mod commands;
pub mod properties;
pub mod property;

pub struct Mpv {
    mpv_handle: *mut ffi::mpv_handle,
    render_ctx: *mut ffi::mpv_render_context,
    pix_buf: Vec<u8>,
    idle: bool,
}

impl Mpv {
    pub fn new() -> anyhow::Result<Self> {
        let mpv_handle = unsafe { ffi::mpv_create() };
        if mpv_handle.is_null() {
            panic!("Failed to create mpv instance");
        }
        // Load mpv config, if exists
        if let Some(home) = std::env::home_dir() {
            let conf_path = home.join(".config/mpv/mpv.conf");
            unsafe {
                let result = mpv_load_config_file(
                    mpv_handle,
                    conf_path.as_os_str().as_encoded_bytes().as_ptr() as *const std::ffi::c_char,
                );
                if result != 0 {
                    eprintln!("Error when loading config file (code {result})");
                }
            }
        }
        let render_ctx = unsafe {
            // If we don't set "libmpv" as video output, mpv opens its own window.
            ffi::mpv_set_option_string(mpv_handle, c"vo".as_ptr(), c"libmpv".as_ptr());
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
            idle: false,
        })
    }
    pub fn command_async<C: Command>(&mut self, command: C)
    where
        [(); C::ARGS_COUNT + 2]:,
    {
        let mut args_buf = [std::ptr::null(); C::ARGS_COUNT + 2];
        args_buf[0] = C::NAME.as_ptr();
        let args = command.args();
        for (i, arg) in args.iter().enumerate() {
            args_buf[i + 1] = arg.as_ptr();
        }
        *args_buf.last_mut().unwrap() = std::ptr::null();
        unsafe {
            ffi::mpv_command_async(self.mpv_handle, 0, args_buf.as_mut_ptr());
        }
    }
    pub fn get_frame_as_pixels(&mut self, present_dim: VideoDim<Present>) -> &[u8] {
        let pix_size = present_dim.rgba_bytes_len();
        if self.pix_buf.len() != pix_size {
            self.pix_buf.resize(pix_size, 0);
        }
        unsafe {
            let mut size: [c_int; 2] = [c_int::from(present_dim.x), c_int::from(present_dim.y)];
            let mut format = *b"rgb0\0";
            let mut stride: usize = present_dim.x as usize * 4;
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
    pub fn get_property<P: Property>(&self) -> Option<P::Type> {
        let mut out: MaybeUninit<<P::Type as PropertyType>::CType> = MaybeUninit::uninit();
        unsafe {
            if ffi::mpv_get_property(
                self.mpv_handle,
                P::NAME.as_ptr(),
                <P::Type as PropertyType>::CType::FORMAT,
                out.as_mut_ptr().cast(),
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
    pub fn set_property<P: PropertyWrite>(&self, value: P::Type) -> bool {
        let mut ret = false;
        value.with_c(|mut cvalue| unsafe {
            ret = ffi::mpv_set_property(
                self.mpv_handle,
                P::NAME.as_ptr(),
                <P::Type as PropertyType>::CType::FORMAT,
                (&mut cvalue) as *mut _ as *mut c_void,
            ) >= 0
        });
        ret
    }

    pub fn unset_property<P: PropertyUnset>(&self) {
        P::UNSET_VALUE.with_c(|mut cvalue| unsafe {
            ffi::mpv_set_property(
                self.mpv_handle,
                P::NAME.as_ptr(),
                <P::UnsetType as PropertyType>::CType::FORMAT,
                (&mut cvalue) as *mut _ as *mut c_void,
            );
        });
    }

    #[must_use]
    pub fn poll_and_handle_event(&mut self) -> Option<MpvEvent> {
        unsafe {
            let ev_ptr = ffi::mpv_wait_event(self.mpv_handle, 0.0);
            if let Some(ev) = ev_ptr.as_ref() {
                let event = match ev.event_id {
                    ffi::mpv_event_id_MPV_EVENT_VIDEO_RECONFIG => MpvEvent::VideoReconfig,
                    ffi::mpv_event_id_MPV_EVENT_FILE_LOADED => MpvEvent::FileLoaded,
                    ffi::mpv_event_id_MPV_EVENT_IDLE => {
                        self.idle = true;
                        MpvEvent::Idle
                    }
                    ffi::mpv_event_id_MPV_EVENT_NONE => return None,
                    ffi::mpv_event_id_MPV_EVENT_SEEK => MpvEvent::Seek,
                    ffi::mpv_event_id_MPV_EVENT_PLAYBACK_RESTART => {
                        self.idle = false;
                        MpvEvent::PlaybackRestart
                    }
                    eid => {
                        eprintln!("Unhandled event id: {eid}");
                        return None;
                    }
                };
                return Some(event);
            }
        }
        None
    }
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.idle
    }
}

pub enum MpvEvent {
    VideoReconfig,
    Idle,
    PlaybackRestart,
    FileLoaded,
    Seek,
}

impl Drop for Mpv {
    fn drop(&mut self) {
        unsafe {
            ffi::mpv_render_context_free(self.render_ctx);
            ffi::mpv_destroy(self.mpv_handle);
        }
    }
}
