use {
    crate::{
        InteractState, MOUSE_OVERLAY_PREFIX, RectDragStatus, SourceMarkers, TabOpen,
        config::Config,
        coords::{VideoDim, VideoMag, VideoPos, VideoVector},
        mpv::{Mpv, MpvEvent, commands as c, properties as p},
        overlay::{self, draw_overlay},
        present::Present,
        sfml_integ::VideoPosSfExt as _,
        subs::SubsState,
        ui::UiState,
    },
    egui_sfml::{
        self, SfEgui,
        sfml::{
            cpp::FBox,
            graphics::{
                Color, Font, Rect, RenderTarget as _, RenderWindow, Sprite, Transformable as _,
                View,
            },
            window::{ContextSettings, Event, Key, Style, mouse},
        },
    },
    std::{fmt::Write as _, path::Path},
};

pub struct App {
    pub mpv: Mpv,
    pub rw: FBox<RenderWindow>,
    pub sf_egui: SfEgui,
    pub ui_state: UiState,
    pub state: AppState,
    pub cfg: Config,
}

/// The "independent" application state that we store on our side
pub struct AppState {
    pub source_markers: SourceMarkers,
    pub interact: InteractState,
    pub subs: Option<SubsState>,
    pub src: crate::source::Info,
    pub present: Option<Present>,
    pub video_area_max_dim: VideoDim<crate::coords::Present>,
    pub pos_string: String,
    pub overlay_show: bool,
}

pub fn load_kashimark_subs(path: &Path, sub_timing_path: Option<&String>) -> SubsState {
    let lines = kashimark::parse(&std::fs::read_to_string(path).unwrap());
    let mut subs = SubsState::new(lines);
    if let Some(path) = sub_timing_path {
        subs.load_timings(path.clone());
    }
    subs
}

impl AppState {
    fn new(args: &crate::Args) -> Self {
        Self {
            subs: args
                .sub
                .as_ref()
                .map(|path| load_kashimark_subs(path.as_ref(), args.sub_timing.as_ref())),
            source_markers: SourceMarkers::default(),
            interact: InteractState::default(),
            src: crate::source::Info {
                dim: VideoDim::new(0, 0),
                w_h_ratio: 1.0,
                duration: 0.0,
                time_pos: 0.0,
                path: String::new(),
            },
            present: None,
            video_area_max_dim: VideoDim::<crate::coords::Present>::new(0, 0),
            pos_string: String::from(MOUSE_OVERLAY_PREFIX),
            overlay_show: true,
        }
    }
}

impl App {
    pub fn handle_keypress(&mut self, code: Key, ctrl: bool) {
        let egui_ctx = self.sf_egui.context();
        if egui_ctx.wants_keyboard_input()
            || self.ui_state.file_dialog.state() == egui_file_dialog::DialogState::Open
        {
            return;
        }
        match code {
            Key::Escape => self.rw.close(),
            Key::Tab => self.state.overlay_show ^= true,
            Key::Space => {
                let pause_flag = self.mpv.get_property::<p::Pause>().unwrap_or(false);
                if !pause_flag {
                    self.mpv.set_property::<p::Pause>(true);
                } else {
                    self.mpv.set_property::<p::Pause>(false);
                }
            }
            Key::Period => self.mpv.command_async(c::FrameStep),
            Key::Comma => self.mpv.command_async(c::FrameBackStep),
            Key::A => {
                if let Some(subs) = &mut self.state.subs {
                    subs.time_stamps
                        .push(self.mpv.get_property::<p::TimePos>().unwrap());
                }
            }
            Key::P => self.mpv.command_async(c::PlaylistPlay::Current),
            Key::S => self.mpv.command_async(c::PlaylistPlay::None),
            Key::O if ctrl => {
                self.ui_state.file_dialog.pick_file();
            }
            Key::R => {
                self.mpv.command_async(c::PlaylistPlay::Index(0));
                if let Some(subs) = &mut self.state.subs {
                    subs.rewind();
                }
            }
            Key::Left => self.mpv.command_async(c::SeekRelSeconds(-10.)),
            Key::Right => self.mpv.command_async(c::SeekRelSeconds(10.)),
            Key::Up => self.mpv.command_async(c::SeekRelSeconds(-30.)),
            Key::Down => self.mpv.command_async(c::SeekRelSeconds(30.)),
            Key::F2 => {
                if let Some(subs) = &mut self.state.subs {
                    subs.save_state();
                }
            }
            Key::F4 => {
                if let Some(subs) = &mut self.state.subs {
                    subs.reload_state();
                    let stamp = *subs.time_stamps.last().unwrap_or(&0.);
                    self.mpv.set_property::<p::TimePos>(stamp);
                }
            }
            _ => {}
        }
    }

    pub fn do_frame(&mut self, font: &Font) {
        if let Some(ev) = self.mpv.poll_and_handle_event() {
            match ev {
                MpvEvent::VideoReconfig => {
                    let actual_video_w = self.mpv.get_property::<p::Width>().unwrap_or(0);
                    let actual_video_h = self.mpv.get_property::<p::Height>().unwrap_or(0);
                    self.state.src.dim =
                        VideoDim::new(actual_video_w as VideoMag, actual_video_h as VideoMag);
                    eprintln!("Video reconfig {:#?}", self.state.src.dim);
                    self.state.present = Present::new(self.state.src.dim.as_present());
                    self.state.src.w_h_ratio = actual_video_w as f64 / actual_video_h as f64
                }
                MpvEvent::Idle | MpvEvent::PlaybackRestart => {}
            }
        }
        let mut collected_events = Vec::new();
        while let Some(event) = self.rw.poll_event() {
            self.sf_egui.add_event(&event);
            if !self.handle_immediate_event(event) {
                collected_events.push(event);
            }
        }
        if let Some(subs) = &mut self.state.subs
            && let Some(current_pos) = self.mpv.get_property::<p::TimePos>()
            && subs
                .time_stamps
                .get(subs.tracking.timestamp_tracker)
                .is_some_and(|pos| current_pos >= *pos)
        {
            subs.advance();
            subs.tracking.timestamp_tracker += 1;
        }
        let raw_mouse_pos = self.rw.mouse_position();
        let src_mouse_pos = VideoPos::from_present(
            raw_mouse_pos.x,
            raw_mouse_pos.y,
            self.state.src.dim,
            self.state
                .present
                .as_mut()
                .map_or(VideoVector::new(0, 0), |present| present.dim),
        );
        self.state.src.duration = self.mpv.get_property::<p::Duration>().unwrap_or(0.0);
        self.state.src.time_pos = self.mpv.get_property::<p::TimePos>().unwrap_or(0.0);
        if let Some(drag) = &self.state.interact.rect_drag {
            match drag.status {
                RectDragStatus::Init => {}
                RectDragStatus::ClickedTopLeft => {
                    self.state.source_markers.rects[drag.idx].rect.dim.x =
                        src_mouse_pos.x - self.state.source_markers.rects[drag.idx].rect.pos.x;
                    self.state.source_markers.rects[drag.idx].rect.dim.y =
                        src_mouse_pos.y - self.state.source_markers.rects[drag.idx].rect.pos.y;
                }
            }
        }
        if let Some(orig_cur) = &self.state.interact.pan_cursor_origin
            && let Some(orig_img) = &self.state.interact.pan_image_original_pos
        {
            let diff_x = orig_cur.x - src_mouse_pos.x;
            let diff_y = orig_cur.y - src_mouse_pos.y;
            self.state.interact.pan_pos.x = orig_img.x - diff_x;
            self.state.interact.pan_pos.y = orig_img.y - diff_y;
        }
        let di = self
            .sf_egui
            .run(&mut self.rw, |_rw, ctx| {
                crate::ui::ui(
                    ctx,
                    &mut self.mpv,
                    &mut self.state,
                    &mut self.ui_state,
                    &mut self.cfg,
                )
            })
            .unwrap();
        // We wait until the egui ui has run, so we know if it wanted input or not
        let wants_kb = self.sf_egui.context().wants_keyboard_input();
        let wants_ptr = self.sf_egui.context().wants_pointer_input();
        for event in collected_events {
            self.handle_delayed_event(event, wants_kb, wants_ptr);
        }

        self.state.pos_string.truncate(MOUSE_OVERLAY_PREFIX.len());
        write!(
            &mut self.state.pos_string,
            "{}, {}",
            src_mouse_pos.x, src_mouse_pos.y,
        )
        .unwrap();
        self.rw.clear(Color::BLACK);
        if let Some(present) = self.state.present.as_mut() {
            let pixels = self.mpv.get_frame_as_pixels(present.dim);
            present.texture.update_from_pixels(
                pixels,
                present.dim.x.try_into().unwrap(),
                present.dim.y.try_into().unwrap(),
                0,
                0,
            );
            let mut s = Sprite::with_texture(&present.texture);
            s.set_position(self.state.interact.pan_pos.to_sf());
            self.rw.draw(&s);
        }
        if self.state.overlay_show {
            draw_overlay(&mut self.rw, &self.state, &self.state.pos_string, font);
        }
        self.sf_egui.draw(di, &mut self.rw, None);
        self.rw.display();
    }

    /// Handle events before the egui ui
    ///
    /// Returns `true` if the event was "used" (don't push it to delayed events)
    #[must_use]
    fn handle_immediate_event(&mut self, event: Event) -> bool {
        match event {
            Event::Closed => self.rw.close(),
            Event::Resized { width, height } => {
                let view = View::from_rect(Rect::new(0., 0., width as f32, height as f32)).unwrap();
                self.rw.set_view(&view);
            }
            _ => return false,
        }
        true
    }

    /// Handle events after the egui ui
    fn handle_delayed_event(&mut self, event: Event, wants_kb: bool, wants_ptr: bool) {
        overlay::handle_event(
            &event,
            &self.mpv,
            &self.state.src,
            self.state.video_area_max_dim,
        );
        match event {
            Event::KeyPressed { code, ctrl, .. } => {
                if !wants_kb {
                    self.handle_keypress(code, ctrl);
                }
            }

            Event::MouseButtonPressed {
                button: mouse::Button::Left,
                x,
                y,
            } => 'block: {
                let Some(present) = self.state.present.as_ref() else {
                    break 'block;
                };
                if wants_ptr {
                    break 'block;
                }
                let pos = VideoPos::from_present(x, y, self.state.src.dim, present.dim);
                if let Some(drag) = &mut self.state.interact.rect_drag {
                    match drag.status {
                        RectDragStatus::Init => {
                            self.state.source_markers.rects[drag.idx].rect.pos = pos;
                            drag.status = RectDragStatus::ClickedTopLeft;
                        }
                        RectDragStatus::ClickedTopLeft => {}
                    }
                } else {
                    self.state.interact.pan_cursor_origin = Some(pos);
                    self.state.interact.pan_image_original_pos = Some(self.state.interact.pan_pos);
                }
            }
            Event::MouseButtonReleased {
                button: mouse::Button::Left,
                x,
                y,
            } => 'block: {
                let Some(present) = self.state.present.as_ref() else {
                    break 'block;
                };
                if wants_ptr {
                    break 'block;
                }
                let pos = VideoPos::from_present(x, y, self.state.src.dim, present.dim);
                if let Some(drag) = &self.state.interact.rect_drag {
                    match drag.status {
                        RectDragStatus::Init => {}
                        RectDragStatus::ClickedTopLeft => {
                            let rect = &mut self.state.source_markers.rects[drag.idx].rect;
                            rect.dim.x = pos.x - rect.pos.x;
                            rect.dim.y = pos.y - rect.pos.y;
                            if rect.pos.x + rect.dim.x > self.state.src.dim.x {
                                let diff = self.state.src.dim.x - rect.pos.x;
                                rect.dim.x = diff;
                            }
                            if rect.pos.y + rect.dim.y > self.state.src.dim.y {
                                let diff = self.state.src.dim.y - rect.pos.y;
                                rect.dim.y = diff;
                            }
                            self.state.interact.rect_drag = None;
                        }
                    }
                }
                self.state.interact.pan_cursor_origin = None;
            }
            _ => {}
        }
    }

    pub fn new(args: &crate::Args, cfg: Config) -> Self {
        let rw = RenderWindow::new(
            (960, 600),
            "ffmpeg-egui",
            Style::RESIZE,
            &ContextSettings::default(),
        )
        .unwrap();
        let sf_egui = SfEgui::new(&rw);
        let mut ui_state = UiState::default();
        if let Some(preset) = &args.ffmpeg_preset {
            ui_state.ffmpeg_cli.source_string = preset.clone();
        }
        if args.open_cli_win {
            ui_state.ffmpeg_cli.open = true;
        }
        if let Some(tab) = args.tab {
            match tab {
                TabOpen::Rects => ui_state.tab = crate::ui::Tab::Rects,
                TabOpen::Timespans => ui_state.tab = crate::ui::Tab::TimeSpans,
            }
        }
        Self {
            state: AppState::new(args),
            mpv: Mpv::new().unwrap(),
            rw,
            sf_egui,
            ui_state: UiState::default(),
            cfg,
        }
    }

    pub(crate) fn save_cfg(&self) {
        self.cfg.save().unwrap();
    }
}
