#![feature(array_chunks)]

use std::{
    ffi::{CStr, CString},
    os::raw::c_int,
};

use egui_sfml::{egui, SfEgui};
use libmpv_sys as mpv;
use mpv::{mpv_error_str, mpv_render_context_render, mpv_render_param};
use sfml::{
    graphics::{Color, Font, RenderTarget, RenderWindow, Sprite, Text, Texture},
    window::{ContextSettings, Event, Key, Style},
};

fn main() {
    let path = std::env::args().nth(1).expect("Need path to media file");
    let path = CString::new(path).unwrap();
    let mut rw = RenderWindow::new(
        (800, 600),
        "ffmpeg-egui",
        Style::RESIZE,
        &ContextSettings::default(),
    );
    rw.set_framerate_limit(60);
    let mut sf_egui = SfEgui::new(&rw);
    let mpv_handle = unsafe { mpv::mpv_create() };
    if mpv_handle.is_null() {
        panic!("Failed to create mpv instance");
    }
    let render_ctx = unsafe {
        if mpv::mpv_initialize(mpv_handle) < 0 {
            panic!("Failed to initialize mpv");
        }
        let mut ctx = std::ptr::null_mut();
        let sw_render_param: &[u8; 3] = mpv::MPV_RENDER_API_TYPE_SW;
        let mut ctrl_param: std::os::raw::c_int = 1;
        let mut params = [
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
                data: sw_render_param.as_ptr() as _,
            },
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_ADVANCED_CONTROL,
                data: (&mut ctrl_param) as *mut _ as _,
            },
            std::mem::zeroed(),
        ];
        let ret_val = mpv::mpv_render_context_create(&mut ctx, mpv_handle, params.as_mut_ptr());
        if ret_val < 0 {
            panic!(
                "Failed to init render context: {}",
                mpv::mpv_error_str(ret_val)
            );
        }
        ctx
    };
    assert!(!render_ctx.is_null());
    unsafe {
        let mut cmd = [b"loadfile\0".as_ptr() as _, path.as_ptr(), std::ptr::null()];
        mpv::mpv_command_async(mpv_handle, 0, cmd.as_mut_ptr());
    }

    let mut tex = Texture::new().unwrap();
    if !tex.create(800, 600) {
        panic!("Failed to create texture");
    }

    let mut pix_buf = [0u8; 800 * 600 * 4];

    let font = unsafe { Font::from_memory(include_bytes!("../DejaVuSansMono.ttf")).unwrap() };
    let prefix = "SFML Overlay: ";
    let mut pos_string = String::from(prefix);
    let mut overlay_show = true;

    while rw.is_open() {
        while let Some(event) = rw.poll_event() {
            sf_egui.add_event(&event);
            match event {
                Event::Closed => rw.close(),
                Event::KeyPressed { code, .. } => match code {
                    Key::Escape => rw.close(),
                    Key::Tab => overlay_show ^= true,
                    _ => {}
                },
                _ => {}
            }
        }
        sf_egui.do_frame(|ctx| {
            egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                ui.label("Hello from egui");
            });
        });
        rw.clear(Color::BLACK);
        unsafe {
            let mut size: [c_int; 2] = [800, 600];
            let mut format = *b"rgb0\0";
            let mut stride: usize = 800 * 4;
            let mut params = [
                mpv_render_param {
                    type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_SW_SIZE,
                    data: size.as_mut_ptr() as _,
                },
                mpv_render_param {
                    type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_SW_FORMAT,
                    data: format.as_mut_ptr() as _,
                },
                mpv_render_param {
                    type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_SW_STRIDE,
                    data: (&mut stride) as *mut _ as _,
                },
                mpv_render_param {
                    type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_SW_POINTER,
                    data: pix_buf.as_mut_ptr() as _,
                },
                std::mem::zeroed(),
            ];
            let result = mpv_render_context_render(render_ctx, params.as_mut_ptr());
            let c_str = mpv::mpv_get_property_string(mpv_handle, b"time-pos\0".as_ptr() as _);
            if c_str.is_null() {
                eprintln!("Couldn't get property string");
            } else {
                pos_string.truncate(prefix.len());
                pos_string.push_str(CStr::from_ptr(c_str).to_str().unwrap());
            }
            mpv::mpv_free(c_str as _);
            for [.., a] in pix_buf.array_chunks_mut::<4>() {
                *a = 255;
            }
            if result < 0 {
                eprintln!("Render error: {}", mpv_error_str(result));
            }
            tex.update_from_pixels(&pix_buf, 800, 600, 0, 0);
        }
        rw.draw(&Sprite::with_texture(&tex));
        if overlay_show {
            rw.draw(&Text::new(&pos_string, &font, 32));
        }
        sf_egui.draw(&mut rw, None);
        rw.display();
    }

    unsafe {
        mpv::mpv_render_context_free(render_ctx);
        mpv::mpv_destroy(mpv_handle);
    }
}
