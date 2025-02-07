#![feature(array_chunks, generic_const_exprs, let_chains)]

use {
    crate::mpv::properties::{CropH, CropW, CropY, Rotate},
    clap::Parser,
    coords::{Src, VideoDim, VideoMag, VideoPos, VideoRect},
    egui_sfml::{
        SfEgui,
        egui::{self},
        sfml::{
            graphics::{
                Color, Font, Rect, RenderTarget, RenderWindow, Sprite, Transformable, View,
            },
            window::{ContextSettings, Event, Key, Style, mouse},
        },
    },
    mpv::{
        Mpv,
        commands::{FrameBackStep, FrameStep, LoadFile, PlaylistPlay, SeekRelSeconds},
        properties::{
            AudioPitchCorrection, CropX, Duration, Height, KeepOpen, KeepOpenPause, Pause, TimePos,
            Volume, Width,
        },
        property::{YesNo, YesNoAlways},
    },
    overlay::draw_overlay,
    present::Present,
    sfml_integ::VideoPosSfExt as _,
    std::fmt::Write,
    subs::SubsState,
    ui::{EguiFriendlyColor, UiState},
};

mod coords;
mod ffmpeg;
mod mpv;
mod overlay;
mod present;
mod sfml_integ;
mod source;
mod subs;
mod time_fmt;
mod ui;

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

struct InteractState {
    rect_drag: Option<RectDrag>,
    pan_cursor_origin: Option<VideoPos<Src>>,
    pan_image_original_pos: Option<VideoPos<Src>>,
    pan_pos: VideoPos<Src>,
}

impl Default for InteractState {
    fn default() -> Self {
        Self {
            rect_drag: Default::default(),
            pan_cursor_origin: Default::default(),
            pan_image_original_pos: Default::default(),
            pan_pos: VideoPos::new(0, 0),
        }
    }
}

struct TimeSpan {
    begin: f64,
    end: f64,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum TabOpen {
    Rects,
    Timespans,
}

#[derive(clap::Parser)]
struct Args {
    /// File to open. File picker will open if not supplied.
    file: Option<String>,
    /// Preset the contents of the FFmpeg CLI input
    #[arg(long)]
    ffmpeg_preset: Option<String>,
    /// Start with FFmpeg CLI window open
    #[arg(long)]
    open_cli_win: bool,
    /// Start with a tab open
    #[arg(long)]
    tab: Option<TabOpen>,
    /// Optional kashimark subtitle file to sync against lyrics
    #[arg(long)]
    sub: Option<String>,
    /// Optional timing file for subtitle
    #[arg(long)]
    sub_timing: Option<String>,
    /// Path to optional overlay font to use instead of default
    #[arg(long)]
    font: Option<String>,
}

fn main() {
    let args = Args::parse();
    let mut mpv = Mpv::new().unwrap();
    let path = match args.file {
        Some(path) => path,
        None => match rfd::FileDialog::new().pick_file() {
            Some(path) => path.to_string_lossy().into_owned(),
            None => return,
        },
    };
    let mut subs_state = match args.sub {
        Some(path) => {
            let lines = kashimark::parse(&std::fs::read_to_string(path).unwrap());
            let mut subs = SubsState::new(lines);
            if let Some(path) = args.sub_timing {
                subs.load_timings(path);
            }
            Some(subs)
        }
        None => None,
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
    )
    .unwrap();
    rw.set_framerate_limit(60);
    let mut interact_state = InteractState::default();
    let mut sf_egui = SfEgui::new(&rw);

    let font = match args.font {
        Some(path) => Font::from_file(&path).unwrap(),
        None => Font::from_memory_static(include_bytes!("../DejaVuSansMono.ttf")).unwrap(),
    };
    let prefix = "Mouse video pos: ";
    let mut pos_string = String::from(prefix);
    let mut overlay_show = true;
    let actual_video_w = mpv.get_property::<Width>().unwrap_or(0);
    let actual_video_h = mpv.get_property::<Height>().unwrap_or(0);
    let crop_x = mpv.get_property::<CropX>().unwrap_or(0);
    let crop_y = mpv.get_property::<CropY>().unwrap_or(0);
    let crop_w = mpv.get_property::<CropW>().unwrap_or(0);
    let crop_h = mpv.get_property::<CropH>().unwrap_or(0);
    let rotate = mpv.get_property::<Rotate>().unwrap_or(0);
    dbg!(crop_x, crop_y, crop_w, crop_h, rotate);
    if rotate != 0 {
        eprintln!("Rotated videos are currently unsupported");
        return;
    }
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
    if let Some(preset) = args.ffmpeg_preset {
        ui_state.ffmpeg_cli.source_string = preset;
    }
    if args.open_cli_win {
        ui_state.ffmpeg_cli.open = true;
    }
    if let Some(tab) = args.tab {
        match tab {
            TabOpen::Rects => ui_state.tab = ui::Tab::Rects,
            TabOpen::Timespans => ui_state.tab = ui::Tab::TimeSpans,
        }
    }

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
                    subs_state.as_mut(),
                ),
                Event::Resized { width, height } => {
                    let view =
                        View::from_rect(Rect::new(0., 0., width as f32, height as f32)).unwrap();
                    rw.set_view(&view);
                }
                Event::MouseButtonPressed {
                    button: mouse::Button::Left,
                    x,
                    y,
                } => 'block: {
                    if sf_egui.context().wants_pointer_input() {
                        break 'block;
                    }
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
        if let Some(subs) = &mut subs_state
            && let Some(current_pos) = mpv.get_property::<TimePos>()
            && subs
                .time_stamps
                .get(subs.tracking.timestamp_tracker)
                .is_some_and(|pos| current_pos >= *pos)
        {
            subs.advance();
            subs.tracking.timestamp_tracker += 1;
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
            let diff_x = orig_cur.x - src_mouse_pos.x;
            let diff_y = orig_cur.y - src_mouse_pos.y;
            interact_state.pan_pos.x = orig_img.x - diff_x;
            interact_state.pan_pos.y = orig_img.y - diff_y;
        }
        let di = sf_egui
            .run(&mut rw, |_rw, ctx| {
                ui::ui(
                    ctx,
                    &mpv,
                    &mut video_area_max_dim,
                    &mut present,
                    &mut source_markers,
                    &src_info,
                    &mut interact_state,
                    &mut ui_state,
                    subs_state.as_mut(),
                )
            })
            .unwrap();
        pos_string.truncate(prefix.len());
        write!(&mut pos_string, "{}, {}", src_mouse_pos.x, src_mouse_pos.y,).unwrap();
        rw.clear(Color::BLACK);

        let pixels = mpv.get_frame_as_pixels(present.dim);
        present.texture.update_from_pixels(
            pixels,
            present.dim.x.try_into().unwrap(),
            present.dim.y.try_into().unwrap(),
            0,
            0,
        );
        let mut s = Sprite::with_texture(&present.texture);
        s.set_position(interact_state.pan_pos.to_sf());
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
                subs_state.as_ref(),
            );
        }
        sf_egui.draw(di, &mut rw, None);
        rw.display();
    }
}

fn handle_keypress(
    code: Key,
    rw: &mut RenderWindow,
    overlay_show: &mut bool,
    mpv: &mut Mpv,
    egui_ctx: &egui::Context,
    subs: Option<&mut SubsState>,
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
        Key::A => {
            if let Some(subs) = subs {
                subs.time_stamps
                    .push(mpv.get_property::<TimePos>().unwrap());
            }
        }
        Key::P => mpv.command_async(PlaylistPlay::Current),
        Key::S => mpv.command_async(PlaylistPlay::None),
        Key::R => {
            mpv.command_async(PlaylistPlay::Index(0));
            if let Some(subs) = subs {
                subs.rewind();
            }
        }
        Key::Left => mpv.command_async(SeekRelSeconds(-10.)),
        Key::Right => mpv.command_async(SeekRelSeconds(10.)),
        Key::Up => mpv.command_async(SeekRelSeconds(-30.)),
        Key::Down => mpv.command_async(SeekRelSeconds(30.)),
        Key::F2 => {
            if let Some(subs) = subs {
                subs.save_state();
            }
        }
        Key::F4 => {
            if let Some(subs) = subs {
                subs.reload_state();
                let stamp = *subs.time_stamps.last().unwrap_or(&0.);
                mpv.set_property::<TimePos>(stamp);
            }
        }
        _ => {}
    }
}
