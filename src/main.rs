#![feature(array_chunks, generic_const_exprs, lint_reasons, let_chains)]

mod coords;
mod ffmpeg;
mod mpv;
mod overlay;
mod present;
mod source;
mod time_fmt;
mod ui;

use coords::{Src, VideoDim, VideoMag, VideoPos, VideoRect};
use egui_sfml::{egui, SfEgui};
use overlay::draw_overlay;
use present::Present;
use std::fmt::Write;
use ui::{EguiFriendlyColor, UiState};

use mpv::{
    commands::{FrameBackStep, FrameStep, LoadFile, PlaylistPlay, SeekRelSeconds},
    properties::{
        AudioPitchCorrection, Duration, Height, KeepOpen, KeepOpenPause, Pause, TimePos, Volume,
        Width,
    },
    property::{YesNo, YesNoAlways},
    Mpv,
};
use sfml::{
    graphics::{Color, Font, Rect, RenderTarget, RenderWindow, Sprite, Transformable, View},
    system::Vector2f,
    window::{mouse, ContextSettings, Event, Key, Style},
};

struct RectDrag {
    idx: usize,
    status: RectDragStatus,
}

struct RectMarker {
    rect: VideoRect<Src>,
    name: String,
    color: EguiFriendlyColor,
}

struct TimespanMarker {
    timespan: TimeSpan,
    name: String,
    color: EguiFriendlyColor,
}

#[derive(Default)]
struct SourceMarkers {
    rects: Vec<RectMarker>,
    timespans: Vec<TimespanMarker>,
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
    pan_cursor_origin: Option<VideoPos<Src>>,
    pan_image_original_pos: Option<Vector2f>,
    pan_pos: Vector2f,
}

struct TimeSpan {
    begin: f64,
    end: f64,
}

fn main() {
    let mut mpv = Mpv::new().unwrap();
    let path = match std::env::args().nth(1) {
        Some(path) => path,
        None => match rfd::FileDialog::new().pick_file() {
            Some(path) => path.to_string_lossy().into_owned(),
            None => return,
        },
    };
    mpv.set_property::<AudioPitchCorrection>(false);
    mpv.set_property::<KeepOpen>(YesNoAlways::Yes);
    mpv.set_property::<KeepOpenPause>(YesNo::No);
    mpv.set_property::<Volume>(75.0);
    mpv.command_async(LoadFile { path: &path });
    let mut source_markers = SourceMarkers::default();
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
        time_pos: 0.0,
        path: path.to_owned(),
    };
    let mut present = Present::new(src_info.dim.as_present());
    let mut ui_state = UiState::default();

    let mut video_area_max_dim = VideoDim::<coords::Present>::new(0, 0);

    while rw.is_open() {
        while let Some(event) = rw.poll_event() {
            sf_egui.add_event(&event);
            overlay::handle_event(&event, &mpv, &src_info, video_area_max_dim);
            match event {
                Event::Closed => rw.close(),
                Event::KeyPressed { code, .. } => handle_keypress(
                    code,
                    &mut rw,
                    &mut overlay_show,
                    &mut mpv,
                    sf_egui.context(),
                ),
                Event::Resized { width, height } => {
                    let view = View::from_rect(Rect::new(0., 0., width as f32, height as f32));
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
                                source_markers.rects[drag.idx].rect.pos = pos;
                                drag.status = RectDragStatus::ClickedTopLeft;
                            }
                            RectDragStatus::ClickedTopLeft => {}
                        }
                    } else {
                        interact_state.pan_cursor_origin = Some(pos);
                        interact_state.pan_image_original_pos = Some(interact_state.pan_pos);
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
                                let rect = &mut source_markers.rects[drag.idx].rect;
                                rect.dim.x = pos.x - rect.pos.x;
                                rect.dim.y = pos.y - rect.pos.y;
                                if rect.pos.x + rect.dim.x > src_info.dim.x {
                                    let diff = src_info.dim.x - rect.pos.x;
                                    rect.dim.x = diff;
                                }
                                if rect.pos.y + rect.dim.y > src_info.dim.y {
                                    let diff = src_info.dim.y - rect.pos.y;
                                    rect.dim.y = diff;
                                }
                                interact_state.rect_drag = None;
                            }
                        }
                    }
                    interact_state.pan_cursor_origin = None;
                }
                _ => {}
            }
        }
        let raw_mouse_pos = rw.mouse_position();
        let src_mouse_pos =
            VideoPos::from_present(raw_mouse_pos.x, raw_mouse_pos.y, src_info.dim, present.dim);
        src_info.duration = mpv.get_property::<Duration>().unwrap_or(0.0);
        src_info.time_pos = mpv.get_property::<TimePos>().unwrap_or(0.0);
        if let Some(drag) = &interact_state.rect_drag {
            match drag.status {
                RectDragStatus::Init => {}
                RectDragStatus::ClickedTopLeft => {
                    source_markers.rects[drag.idx].rect.dim.x =
                        src_mouse_pos.x - source_markers.rects[drag.idx].rect.pos.x;
                    source_markers.rects[drag.idx].rect.dim.y =
                        src_mouse_pos.y - source_markers.rects[drag.idx].rect.pos.y;
                }
            }
        }
        if let Some(orig_cur) = &interact_state.pan_cursor_origin
            && let Some(orig_img) = &interact_state.pan_image_original_pos
        {
            let diff_x = (orig_cur.x - src_mouse_pos.x) as f32;
            let diff_y = (orig_cur.y - src_mouse_pos.y) as f32;
            interact_state.pan_pos.x = orig_img.x + diff_x;
            interact_state.pan_pos.y = orig_img.y + diff_y;
        }
        sf_egui
            .do_frame(|ctx| {
                ui::ui(
                    ctx,
                    &mpv,
                    &mut video_area_max_dim,
                    &mut present,
                    &mut source_markers,
                    &src_info,
                    &mut interact_state,
                    &mut ui_state,
                )
            })
            .unwrap();
        pos_string.truncate(prefix.len());
        write!(&mut pos_string, "{}, {}", src_mouse_pos.x, src_mouse_pos.y,).unwrap();
        rw.clear(Color::BLACK);

        unsafe {
            let pixels = mpv.get_frame_as_pixels(present.dim);
            present.texture.update_from_pixels(
                pixels,
                present.dim.x.try_into().unwrap(),
                present.dim.y.try_into().unwrap(),
                0,
                0,
            );
        }
        let mut s = Sprite::with_texture(&present.texture);
        s.set_position(interact_state.pan_pos);
        rw.draw(&s);
        if overlay_show {
            draw_overlay(
                &mut rw,
                &pos_string,
                &font,
                &source_markers,
                &src_info,
                present.dim,
                video_area_max_dim,
            );
        }
        sf_egui.draw(&mut rw, None);
        rw.display();
    }
}

fn handle_keypress(
    code: Key,
    rw: &mut RenderWindow,
    overlay_show: &mut bool,
    mpv: &mut Mpv,
    egui_ctx: &egui::Context,
) {
    if egui_ctx.wants_keyboard_input() {
        return;
    }
    match code {
        Key::Escape => rw.close(),
        Key::Tab => *overlay_show ^= true,
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
        Key::Left => mpv.command_async(SeekRelSeconds(-10.)),
        Key::Right => mpv.command_async(SeekRelSeconds(10.)),
        Key::Up => mpv.command_async(SeekRelSeconds(-30.)),
        Key::Down => mpv.command_async(SeekRelSeconds(30.)),
        _ => {}
    }
}
