#![feature(array_chunks)]
#![feature(generic_const_exprs, generic_associated_types)]

mod coords;
mod mpv;
mod overlay;
mod time_fmt;
mod ui;

use coords::{video_mouse_pos, VideoDim};
use egui_sfml::SfEgui;
use overlay::draw_overlay;
use std::fmt::Write;

use mpv::{
    commands::{FrameBackStep, FrameStep, LoadFile, PlaylistPlay},
    properties::{AudioPitchCorrection, Duration, Height, KeepOpen, KeepOpenPause, Pause, Width},
    property::{YesNo, YesNoAlways},
    Mpv,
};
use sfml::{
    graphics::{Color, Font, Rect, RenderTarget, RenderWindow, Sprite, Texture, View},
    window::{ContextSettings, Event, Key, Style},
};

struct VideoSrcInfo {
    dim: VideoDim,
    w_h_ratio: f64,
    duration: f64,
}

fn main() {
    let path = std::env::args().nth(1).expect("Need path to media file");
    let mut mpv = Mpv::new().unwrap();
    mpv.set_property::<AudioPitchCorrection>(false);
    mpv.set_property::<KeepOpen>(YesNoAlways::Yes);
    mpv.set_property::<KeepOpenPause>(YesNo::No);
    mpv.command_async(LoadFile { path: &path });
    let mut rects: Vec<Rect<u16>> = Vec::new();
    let mut rw = RenderWindow::new(
        (960, 600),
        "ffmpeg-egui",
        Style::RESIZE,
        &ContextSettings::default(),
    );
    rw.set_framerate_limit(60);
    let mut sf_egui = SfEgui::new(&rw);

    let font = unsafe { Font::from_memory(include_bytes!("../DejaVuSansMono.ttf")).unwrap() };
    let prefix = "Mouse video pos: ";
    let mut pos_string = String::from(prefix);
    let mut overlay_show = true;
    let actual_video_w = mpv.get_property::<Width>().unwrap();
    let actual_video_h = mpv.get_property::<Height>().unwrap();
    let w_h_ratio = actual_video_w as f64 / actual_video_h as f64;
    let mut src_info = VideoSrcInfo {
        dim: VideoDim {
            width: actual_video_w as u16,
            height: actual_video_h as u16,
        },
        w_h_ratio,
        duration: 0.0,
    };
    let mut video_present_dim = src_info.dim;
    let mut tex = Texture::new().unwrap();
    if !tex.create(
        video_present_dim.width.into(),
        video_present_dim.height.into(),
    ) {
        panic!("Failed to create texture");
    }
    let mut video_area_max_h = 100.0;

    while rw.is_open() {
        while let Some(event) = rw.poll_event() {
            sf_egui.add_event(&event);
            match event {
                Event::Closed => rw.close(),
                Event::KeyPressed { code, .. } => match code {
                    Key::Escape => rw.close(),
                    Key::Tab => overlay_show ^= true,
                    Key::Space => {
                        let pause_flag = mpv.get_property::<Pause>().unwrap_or(false);
                        if !pause_flag {
                            mpv.set_property::<Pause>(true);
                        } else {
                            mpv.set_property::<Pause>(false);
                        }
                    }
                    Key::Period => mpv.command_async(FrameStep),
                    Key::Comma => mpv.command_async(FrameBackStep),
                    Key::P => mpv.command_async(PlaylistPlay::Current),
                    Key::S => mpv.command_async(PlaylistPlay::None),
                    Key::R => mpv.command_async(PlaylistPlay::Index(0)),
                    _ => {}
                },
                Event::Resized { width, height } => {
                    let view = View::from_rect(&Rect::new(0., 0., width as f32, height as f32));
                    rw.set_view(&view);
                }
                _ => {}
            }
        }
        let mouse_pos = rw.mouse_position();
        src_info.duration = mpv.get_property::<Duration>().unwrap_or(0.0);
        sf_egui.do_frame(|ctx| {
            ui::ui(
                ctx,
                &mut mpv,
                &mut video_present_dim,
                &mut video_area_max_h,
                &mut tex,
                &mut rects,
                &src_info,
            )
        });
        let (mvx, mvy) = video_mouse_pos(mouse_pos, src_info.dim, video_present_dim);
        pos_string.truncate(prefix.len());
        write!(&mut pos_string, "{}, {}", mvx, mvy,).unwrap();
        rw.clear(Color::BLACK);

        unsafe {
            let pixels = mpv.get_frame_as_pixels(video_present_dim);
            tex.update_from_pixels(
                pixels,
                video_present_dim.width.into(),
                video_present_dim.height.into(),
                0,
                0,
            );
        }
        rw.draw(&Sprite::with_texture(&tex));
        if overlay_show {
            draw_overlay(
                &mut rw,
                &pos_string,
                &font,
                &rects,
                &src_info,
                video_present_dim,
            );
        }
        sf_egui.draw(&mut rw, None);
        rw.display();
    }
}
