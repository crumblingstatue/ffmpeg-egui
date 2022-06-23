#![feature(array_chunks)]
#![feature(generic_const_exprs, generic_associated_types)]

mod coords;
mod mpv;
mod overlay;
mod present;
mod source;
mod time_fmt;
mod ui;

use coords::{Src, VideoDim, VideoMag, VideoPos, VideoRect};
use egui_sfml::SfEgui;
use overlay::draw_overlay;
use present::Present;
use std::fmt::Write;

use mpv::{
    commands::{FrameBackStep, FrameStep, LoadFile, PlaylistPlay},
    properties::{AudioPitchCorrection, Duration, Height, KeepOpen, KeepOpenPause, Pause, Width},
    property::{YesNo, YesNoAlways},
    Mpv,
};
use sfml::{
    graphics::{Color, Font, Rect, RenderTarget, RenderWindow, Sprite, View},
    window::{mouse, ContextSettings, Event, Key, Style},
};

struct RectDrag {
    idx: usize,
    status: RectDragStatus,
}

impl RectDrag {
    fn new(idx: usize) -> Self {
        Self {
            idx,
            status: RectDragStatus::Init,
        }
    }
}

enum RectDragStatus {
    Init,
    ClickedTopLeft,
}

#[derive(Default)]
struct InteractState {
    rect_drag: Option<RectDrag>,
}

fn main() {
    let path = std::env::args().nth(1).expect("Need path to media file");
    let mut mpv = Mpv::new().unwrap();
    mpv.set_property::<AudioPitchCorrection>(false);
    mpv.set_property::<KeepOpen>(YesNoAlways::Yes);
    mpv.set_property::<KeepOpenPause>(YesNo::No);
    mpv.command_async(LoadFile { path: &path });
    let mut rects: Vec<VideoRect<Src>> = Vec::new();
    let mut rw = RenderWindow::new(
        (960, 600),
        "ffmpeg-egui",
        Style::RESIZE,
        &ContextSettings::default(),
    );
    rw.set_framerate_limit(60);
    let mut interact_state = InteractState::default();
    let mut sf_egui = SfEgui::new(&rw);

    let font = unsafe { Font::from_memory(include_bytes!("../DejaVuSansMono.ttf")).unwrap() };
    let prefix = "Mouse video pos: ";
    let mut pos_string = String::from(prefix);
    let mut overlay_show = true;
    let actual_video_w = mpv.get_property::<Width>().unwrap();
    let actual_video_h = mpv.get_property::<Height>().unwrap();
    let w_h_ratio = actual_video_w as f64 / actual_video_h as f64;
    let mut src_info = source::Info {
        dim: VideoDim::new(actual_video_w as VideoMag, actual_video_h as VideoMag),
        w_h_ratio,
        duration: 0.0,
    };
    let mut present = Present::new(src_info.dim.as_present());

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
                Event::MouseButtonPressed {
                    button: mouse::Button::Left,
                    x,
                    y,
                } => {
                    let pos = VideoPos::from_present(x, y, src_info.dim, present.dim);
                    if let Some(drag) = &mut interact_state.rect_drag {
                        match drag.status {
                            RectDragStatus::Init => {
                                rects[drag.idx].pos = pos;
                                drag.status = RectDragStatus::ClickedTopLeft;
                            }
                            RectDragStatus::ClickedTopLeft => {}
                        }
                    }
                }
                Event::MouseButtonReleased {
                    button: mouse::Button::Left,
                    x,
                    y,
                } => {
                    let pos = VideoPos::from_present(x, y, src_info.dim, present.dim);
                    if let Some(drag) = &interact_state.rect_drag {
                        match drag.status {
                            RectDragStatus::Init => {}
                            RectDragStatus::ClickedTopLeft => {
                                rects[drag.idx].dim.x = pos.x - rects[drag.idx].pos.x;
                                rects[drag.idx].dim.y = pos.y - rects[drag.idx].pos.y;
                                interact_state.rect_drag = None;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let raw_mouse_pos = rw.mouse_position();
        let src_mouse_pos =
            VideoPos::from_present(raw_mouse_pos.x, raw_mouse_pos.y, src_info.dim, present.dim);
        src_info.duration = mpv.get_property::<Duration>().unwrap_or(0.0);
        if let Some(drag) = &interact_state.rect_drag {
            match drag.status {
                RectDragStatus::Init => {}
                RectDragStatus::ClickedTopLeft => {
                    rects[drag.idx].dim.x = src_mouse_pos.x - rects[drag.idx].pos.x;
                    rects[drag.idx].dim.y = src_mouse_pos.y - rects[drag.idx].pos.y;
                }
            }
        }
        sf_egui.do_frame(|ctx| {
            ui::ui(
                ctx,
                &mut mpv,
                &mut video_area_max_h,
                &mut present,
                &mut rects,
                &src_info,
                &mut interact_state,
            )
        });
        pos_string.truncate(prefix.len());
        write!(&mut pos_string, "{}, {}", src_mouse_pos.x, src_mouse_pos.y,).unwrap();
        rw.clear(Color::BLACK);

        unsafe {
            let pixels = mpv.get_frame_as_pixels(present.dim);
            present.texture.update_from_pixels(
                pixels,
                present.dim.x.into(),
                present.dim.y.into(),
                0,
                0,
            );
        }
        rw.draw(&Sprite::with_texture(&present.texture));
        if overlay_show {
            draw_overlay(&mut rw, &pos_string, &font, &rects, &src_info, present.dim);
        }
        sf_egui.draw(&mut rw, None);
        rw.display();
    }
}
