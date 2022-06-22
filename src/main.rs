#![feature(array_chunks)]
#![feature(generic_const_exprs, generic_associated_types)]

mod mpv;
mod time_fmt;
mod ui;

use egui_sfml::SfEgui;
use std::fmt::Write;

use mpv::{
    commands::{FrameBackStep, FrameStep, LoadFile, PlaylistPlay},
    properties::{AudioPitchCorrection, Duration, Height, KeepOpen, KeepOpenPause, Pause, Width},
    property::{YesNo, YesNoAlways},
    Mpv,
};
use sfml::{
    graphics::{
        Color, Font, Rect, RectangleShape, RenderTarget, RenderWindow, Shape, Sprite, Text,
        Texture, Transformable, View,
    },
    window::{ContextSettings, Event, Key, Style},
};

struct VideoSrcInfo {
    width: u16,
    height: u16,
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

    let mut tex = Texture::new().unwrap();
    let mut video_w: u16 = 800;
    let mut video_h: u16 = 600;
    if !tex.create(video_w.into(), video_h.into()) {
        panic!("Failed to create texture");
    }

    let font = unsafe { Font::from_memory(include_bytes!("../DejaVuSansMono.ttf")).unwrap() };
    let prefix = "Mouse video pos: ";
    let mut pos_string = String::from(prefix);
    let mut overlay_show = true;
    let actual_video_w = mpv.get_property::<Width>().unwrap();
    let actual_video_h = mpv.get_property::<Height>().unwrap();
    let w_h_ratio = actual_video_w as f64 / actual_video_h as f64;
    let mut src_info = VideoSrcInfo {
        width: actual_video_w as u16,
        height: actual_video_h as u16,
        w_h_ratio,
        duration: 0.0,
    };
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
                &mut video_w,
                &mut video_h,
                &mut video_area_max_h,
                &mut tex,
                &mut rects,
                &src_info,
            )
        });
        let (mvx, mvy) =
            video_mouse_pos(mouse_pos, actual_video_w, actual_video_h, video_w, video_h);
        pos_string.truncate(prefix.len());
        write!(&mut pos_string, "{}, {}", mvx, mvy,).unwrap();
        rw.clear(Color::BLACK);

        unsafe {
            let pixels = mpv.get_frame_as_pixels(video_w, video_h);
            tex.update_from_pixels(pixels, video_w.into(), video_h.into(), 0, 0);
        }
        rw.draw(&Sprite::with_texture(&tex));
        if overlay_show {
            rw.draw(&Text::new(&pos_string, &font, 32));
            let mut rs = RectangleShape::default();
            rs.set_fill_color(Color::rgba(250, 250, 200, 128));
            for rect in &rects {
                let (w, h) = translate_up(
                    rect.width as i32,
                    rect.height as i32,
                    actual_video_w,
                    actual_video_h,
                    video_w,
                    video_h,
                );
                rs.set_size((w as f32, h as f32));
                let (x, y) = translate_up(
                    rect.left as i32,
                    rect.top as i32,
                    actual_video_w,
                    actual_video_h,
                    video_w,
                    video_h,
                );
                rs.set_position((x as f32, y as f32));
                rw.draw(&rs);
            }
        }
        sf_egui.draw(&mut rw, None);
        rw.display();
    }
}

fn video_mouse_pos(
    mouse_pos: sfml::system::Vector2<i32>,
    actual_video_w: i64,
    actual_video_h: i64,
    video_w: u16,
    video_h: u16,
) -> (i16, i16) {
    translate_down(
        mouse_pos.x,
        mouse_pos.y,
        actual_video_w,
        actual_video_h,
        video_w,
        video_h,
    )
}

/// window -> vid coords
fn translate_down(
    x: i32,
    y: i32,
    actual_video_w: i64,
    actual_video_h: i64,
    video_w: u16,
    video_h: u16,
) -> (i16, i16) {
    let w_ratio = actual_video_w as f64 / video_w as f64;
    let h_ratio = actual_video_h as f64 / video_h as f64;
    ((x as f64 * w_ratio) as i16, (y as f64 * h_ratio) as i16)
}

/// vid -> window coords
fn translate_up(
    x: i32,
    y: i32,
    actual_video_w: i64,
    actual_video_h: i64,
    video_w: u16,
    video_h: u16,
) -> (i16, i16) {
    let w_ratio = video_w as f64 / actual_video_w as f64;
    let h_ratio = video_h as f64 / actual_video_h as f64;
    ((x as f64 * w_ratio) as i16, (y as f64 * h_ratio) as i16)
}

fn video_pix_size(w: u16, h: u16) -> usize {
    (w as usize * h as usize) * 4
}
