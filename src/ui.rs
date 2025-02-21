use {
    crate::{
        InteractState, RectDrag, RectMarker, SourceMarkers, SubsState, TimeSpan, TimespanMarker,
        coords::{self, VideoDim, VideoMag, VideoPos, VideoRect},
        ffmpeg::{self, resolve_arguments},
        mpv::{
            Mpv,
            properties::{AbLoopA, AbLoopB, AudioId, Speed, SubId, TimePos, Volume},
        },
        present::Present,
        source,
        time_fmt::FfmpegTimeFmt,
    },
    egui_file_dialog::FileDialog,
    egui_sfml::{
        egui::{self, RichText, ScrollArea},
        sfml::graphics::Color,
    },
    rand::Rng,
    std::io::Read,
};

pub struct UiState {
    pub tab: Tab,
    selected_timespan: Option<usize>,
    rename_index: Option<usize>,
    selected_rect: Option<usize>,
    pub ffmpeg_cli: FfmpegCli,
    file_dialog: FileDialog,
}

#[derive(Default)]
pub struct FfmpegCli {
    pub open: bool,
    pub source_string: String,
    pub first_frame: bool,
    child: Option<std::process::Child>,
    err_str: String,
    exit_status: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            tab: Tab::Rects,
            selected_timespan: None,
            rename_index: None,
            selected_rect: None,
            ffmpeg_cli: FfmpegCli::default(),
            file_dialog: FileDialog::new(),
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Tab {
    Rects,
    TimeSpans,
}

impl Tab {
    fn name(&self) -> &'static str {
        match self {
            Tab::Rects => "Rects",
            Tab::TimeSpans => "Time spans",
        }
    }
}

#[expect(clippy::too_many_arguments)]
pub(crate) fn ui(
    ctx: &egui::Context,
    mpv: &mut Mpv,
    video_area_max_dim: &mut VideoDim<coords::Present>,
    present: Option<&mut Present>,
    source_markers: &mut SourceMarkers,
    src_info: &source::Info,
    interact_state: &mut InteractState,
    ui_state: &mut UiState,
    subs: Option<&mut SubsState>,
) {
    let re = egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        bottom_bar_ui(
            ui,
            src_info,
            present,
            mpv,
            video_area_max_dim,
            ui_state,
            interact_state,
            subs,
        );
    });
    video_area_max_dim.y = re.response.rect.top() as VideoMag;
    let re = egui::SidePanel::right("right_panel").show(ctx, |ui| {
        right_panel_ui(ui, ui_state, source_markers, interact_state, src_info, mpv);
    });
    video_area_max_dim.x = re.response.rect.left() as VideoMag;
    if ui_state.ffmpeg_cli.open {
        egui::Window::new("ffmpeg").show(ctx, |ui| {
            ffmpeg_cli_ui(ui, ui_state, source_markers, src_info);
        });
        ui_state.ffmpeg_cli.first_frame = false;
    }
    ui_state.file_dialog.update(ctx);
    if let Some(path) = ui_state.file_dialog.take_picked() {
        mpv.command_async(crate::mpv::commands::LoadFile {
            path: path.to_str().unwrap(),
        });
    }
}

const FFMPEG_HELP_TEXT: &str = "\
{i}: Currently opened media file
{r.x} Rectangle 'x'
{t.x} Timespan 'x'
";

fn ffmpeg_cli_ui(
    ui: &mut egui::Ui,
    ui_state: &mut UiState,
    source_markers: &SourceMarkers,
    src_info: &source::Info,
) {
    let ctrl_enter = ui.input_mut(|inp| inp.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));
    let re = ui.add(
        egui::TextEdit::multiline(&mut ui_state.ffmpeg_cli.source_string)
            .hint_text("arguments to ffmpeg"),
    );
    match resolve_arguments(&ui_state.ffmpeg_cli.source_string, source_markers, src_info) {
        Ok(args) => {
            let mut args_str = String::new();
            for (i, arg) in args.iter().enumerate() {
                args_str.push_str(&format!("{i}: `{arg}`\n"));
            }
            ui.label(RichText::new(args_str).color(egui::Color32::GOLD));
            if ui.button("run (ctrl+enter)").clicked() || ctrl_enter {
                ui_state.ffmpeg_cli.exit_status = None;
                ui_state.ffmpeg_cli.err_str.clear();
                ui_state.ffmpeg_cli.stderr.clear();
                ui_state.ffmpeg_cli.stdout.clear();
                match ffmpeg::invoke(&ui_state.ffmpeg_cli.source_string, source_markers, src_info) {
                    Ok(child) => ui_state.ffmpeg_cli.child = Some(child),
                    Err(e) => ui_state.ffmpeg_cli.err_str = e.to_string(),
                }
            }
        }
        Err(e) => {
            ui.label(RichText::new(e.to_string()).color(egui::Color32::RED));
        }
    }
    if ui_state.ffmpeg_cli.first_frame {
        re.request_focus();
    }
    ui.label(FFMPEG_HELP_TEXT);
    if !ui_state.ffmpeg_cli.err_str.is_empty() {
        ui.label(RichText::new(&ui_state.ffmpeg_cli.err_str).color(egui::Color32::RED));
    }
    if let Some(child) = &mut ui_state.ffmpeg_cli.child {
        ui.horizontal(|ui| {
            ui.label("running ffmpeg");
            if ui.button("kill").clicked() {
                if let Err(e) = child.kill() {
                    eprintln!("Error killing child process: {e}");
                }
            }
            ui.spinner();
        });
        match child.try_wait() {
            Ok(Some(status)) => {
                ui_state.ffmpeg_cli.exit_status = status.code();
                if let Some(mut stdout) = child.stdout.take() {
                    let mut buf = Vec::new();
                    stdout.read_to_end(&mut buf).unwrap();
                    ui_state.ffmpeg_cli.stdout = String::from_utf8_lossy(&buf).into_owned();
                }
                if let Some(mut stderr) = child.stderr.take() {
                    let mut buf = Vec::new();
                    stderr.read_to_end(&mut buf).unwrap();
                    ui_state.ffmpeg_cli.stderr = String::from_utf8_lossy(&buf).into_owned();
                }
                ui_state.ffmpeg_cli.child = None;
            }
            Ok(None) => {}
            Err(e) => ui_state.ffmpeg_cli.err_str = e.to_string(),
        }
    }
    if let Some(code) = ui_state.ffmpeg_cli.exit_status {
        ui.label(format!("Exit status: {}", code));
    }
    if !ui_state.ffmpeg_cli.stdout.is_empty() {
        ui.label("Standard output:");
        ScrollArea::vertical()
            .max_height(400.0)
            .id_salt("stdout")
            .show(ui, |ui| {
                ui.text_edit_multiline(&mut ui_state.ffmpeg_cli.stdout);
            });
    }
    if !ui_state.ffmpeg_cli.stderr.is_empty() {
        ui.label("Standard error:");
        ScrollArea::vertical()
            .max_height(400.0)
            .id_salt("stderr")
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.text_edit_multiline(&mut ui_state.ffmpeg_cli.stderr);
            });
    }
}

fn right_panel_ui(
    ui: &mut egui::Ui,
    ui_state: &mut UiState,
    source_markers: &mut SourceMarkers,
    interact_state: &mut InteractState,
    src_info: &source::Info,
    mpv: &Mpv,
) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.tab, Tab::Rects, Tab::Rects.name());
        ui.selectable_value(&mut ui_state.tab, Tab::TimeSpans, Tab::TimeSpans.name());
    });
    ui.separator();
    match ui_state.tab {
        Tab::Rects => rects_ui(ui, source_markers, interact_state, ui_state),
        Tab::TimeSpans => timespans_ui(ui, source_markers, src_info, ui_state, mpv),
    }
}

#[expect(clippy::too_many_arguments)]
fn bottom_bar_ui(
    ui: &mut egui::Ui,
    src_info: &source::Info,
    present: Option<&mut Present>,
    mpv: &Mpv,
    video_area_max_dim: &VideoDim<coords::Present>,
    ui_state: &mut UiState,
    interact_state: &mut InteractState,
    subs: Option<&mut SubsState>,
) {
    ui.horizontal(|ui| {
        ui.label(format!(
            "{}/{}",
            FfmpegTimeFmt(src_info.time_pos),
            FfmpegTimeFmt(src_info.duration)
        ));
        ui.style_mut().spacing.slider_width = ui.available_width();
        let mut pos = src_info.time_pos;
        if ui
            .add(egui::Slider::new(&mut pos, 0.0..=src_info.duration).show_value(false))
            .changed()
        {
            mpv.set_property::<TimePos>(pos);
        }
    });
    ui.horizontal(|ui| {
        if let Some(mut speed) = mpv.get_property::<Speed>() {
            ui.label("Playback speed");
            if ui.add(egui::Slider::new(&mut speed, 0.1..=2.0)).changed() {
                mpv.set_property::<Speed>(speed);
            }
        }
        if let Some(mut vol) = mpv.get_property::<Volume>() {
            ui.label("Playback volume");
            if ui.add(egui::Slider::new(&mut vol, 0.0..=150.0)).changed() {
                mpv.set_property::<Volume>(vol);
            }
        }
        let f5 = ui.input(|inp| inp.key_pressed(egui::Key::F5));
        if ui
            .selectable_label(ui_state.ffmpeg_cli.open, "ffmpeg cli (F5)")
            .clicked()
            || f5
        {
            ui_state.ffmpeg_cli.open ^= true;
            ui_state.ffmpeg_cli.first_frame = true;
        }
        ui.menu_button("Menu", |ui| {
            if ui.button("Load video...").clicked() {
                ui_state.file_dialog.pick_file();
                ui.close_menu();
            }
            if ui.button("Reset pan").clicked() {
                interact_state.pan_pos = VideoPos::new(0, 0);
                ui.close_menu();
            }
            ui.menu_button("Video size", |ui| {
                let Some(present) = present else {
                    return;
                };
                let mut present_size_changed = false;
                if ui.button("Original").clicked() {
                    present.dim.x = src_info.dim.x as VideoMag;
                    present.dim.y = src_info.dim.y as VideoMag;
                    present_size_changed = true;
                    ui.close_menu();
                }
                if ui.button("Fit").clicked() {
                    present.dim.y = video_area_max_dim.y;
                    present.dim.x = (present.dim.y as f64 * src_info.w_h_ratio) as VideoMag;
                    if present.dim.x > video_area_max_dim.x {
                        present.dim.x = video_area_max_dim.x;
                        present.dim.y = (present.dim.x as f64 / src_info.w_h_ratio) as VideoMag;
                    }
                    present_size_changed = true;
                    ui.close_menu();
                }
                ui.label("Width");
                if ui.add(egui::DragValue::new(&mut present.dim.x)).changed() {
                    present.dim.y = (present.dim.x as f64 / src_info.w_h_ratio) as VideoMag;
                    present_size_changed = true;
                }
                ui.label("Height");
                if ui.add(egui::DragValue::new(&mut present.dim.y)).changed() {
                    present.dim.x = (present.dim.y as f64 * src_info.w_h_ratio) as VideoMag;
                    present_size_changed = true;
                }
                if present_size_changed {
                    // Clamp range to make it somewhat sane
                    dbg!(present.dim);
                    present.dim.x = (present.dim.x).clamp(1, 4096);
                    present.dim.y = (present.dim.y).clamp(1, 4096);
                    dbg!(present.dim);
                    if present
                        .texture
                        .create(
                            (present.dim.x).try_into().unwrap(),
                            (present.dim.y).try_into().unwrap(),
                        )
                        .is_err()
                    {
                        panic!("Failed to create texture");
                    }
                }
            });
            if let Some(mut current) = mpv.get_property::<AudioId>() {
                ui.horizontal(|ui| {
                    ui.label("Audio track");
                    if ui.add(egui::DragValue::new(&mut current)).changed() {
                        mpv.set_property::<AudioId>(current);
                    }
                });
            } else if ui.button("Set audio track to 1").clicked() {
                ui.close_menu();
                mpv.set_property::<AudioId>(1);
            }
            if let Some(mut current) = mpv.get_property::<SubId>() {
                ui.horizontal(|ui| {
                    ui.label("Sub track");
                    if ui.add(egui::DragValue::new(&mut current)).changed() {
                        mpv.set_property::<SubId>(current);
                    }
                });
            } else if ui.button("Set sub track to 1").clicked() {
                ui.close_menu();
                mpv.set_property::<SubId>(1);
            }
            if let Some(subs) = subs {
                if ui.button("Clear sub timings").clicked() {
                    ui.close_menu();
                    subs.clear();
                }
                if ui.button("Save sub timings to file").clicked() {
                    ui.close_menu();
                    subs.save_timings();
                }
                if let Some(reload) = subs.timings_reload_sentry() {
                    if ui.button("Reload sub timings from file").clicked() {
                        ui.close_menu();
                        reload.reload();
                    }
                }
            }
        });
    });
}

fn timespans_ui(
    ui: &mut egui::Ui,
    markers: &mut SourceMarkers,
    src_info: &source::Info,
    ui_state: &mut UiState,
    mpv: &Mpv,
) {
    if ui.button("Add").clicked() {
        markers.timespans.push(TimespanMarker {
            timespan: TimeSpan {
                begin: src_info.time_pos,
                end: src_info.time_pos,
            },
            name: format!("{}", markers.timespans.len()),
            color: random_color(),
        });
        ui_state.selected_timespan = Some(markers.timespans.len().saturating_sub(1));
    }
    ui.separator();
    ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
        let mut i = 0;
        markers.timespans.retain_mut(|marker| {
            let mut retain = true;
            ui.horizontal(|ui| {
                egui::color_picker::color_edit_button_rgb(ui, &mut marker.color);
                if ui_state.rename_index == Some(i) {
                    let re = ui.text_edit_singleline(&mut marker.name);
                    if re.lost_focus() {
                        ui_state.rename_index = None;
                    }
                    re.request_focus();
                } else {
                    let re =
                        ui.selectable_label(ui_state.selected_timespan == Some(i), &marker.name);
                    if re.clicked() {
                        ui_state.selected_timespan = Some(i);
                    }
                }
                if ui.button("🗑").clicked() {
                    if ui_state.selected_timespan == Some(i) {
                        ui_state.selected_timespan = None;
                    }
                    retain = false;
                }
            });
            i += 1;
            retain
        });
    });
    if let Some(timespan_idx) = ui_state.selected_timespan {
        ui.separator();
        let marker = &mut markers.timespans[timespan_idx];
        egui::Grid::new("begin_end_grid").show(ui, |ui| {
            ui.label("begin");
            ui.add(egui::DragValue::new(&mut marker.timespan.begin));
            if ui.button("=").on_hover_text("Set to current").clicked() {
                marker.timespan.begin = src_info.time_pos;
            }
            if ui.button("▶").on_hover_text("Seek here").clicked() {
                mpv.set_property::<TimePos>(marker.timespan.begin);
            }
            ui.end_row();
            ui.label("end");
            ui.add(egui::DragValue::new(&mut marker.timespan.end));
            if ui.button("=").on_hover_text("Set to current").clicked() {
                marker.timespan.end = src_info.time_pos;
            }
            if ui.button(">").on_hover_text("Set to end").clicked() {
                marker.timespan.end = src_info.duration;
            }
            if ui.button("▶").on_hover_text("Seek here").clicked() {
                mpv.set_property::<TimePos>(marker.timespan.end);
            }
            ui.end_row();
            ui.label("Duration");
            let dur_s = format!("{:.03}", marker.timespan.end - marker.timespan.begin);
            ui.label(&dur_s);
            if ui.button("copy").clicked() {
                ui.ctx().copy_text(dur_s);
            }
        });

        if ui.button("Rename (F2)").clicked() || ui.input(|inp| inp.key_pressed(egui::Key::F2)) {
            ui_state.rename_index = Some(timespan_idx);
        }
        if ui.button("A-B loop").clicked() {
            mpv.set_property::<AbLoopA>(marker.timespan.begin);
            mpv.set_property::<AbLoopB>(marker.timespan.end);
            mpv.set_property::<TimePos>(marker.timespan.begin);
        }
    }
    ui.separator();
    let label_string = match (mpv.get_property::<AbLoopA>(), mpv.get_property::<AbLoopB>()) {
        (Some(a), Some(b)) => {
            format!("ab-loop: {}-{}", a, b)
        }
        (Some(a), None) => {
            format!("loop from {}", a)
        }
        (None, Some(b)) => {
            format!("loop to {}", b)
        }
        (None, None) => String::new(),
    };
    if !label_string.is_empty() {
        ui.label(label_string);
        if ui.button("Clear A-B loop").clicked() {
            mpv.unset_property::<AbLoopA>();
            mpv.unset_property::<AbLoopB>();
        }
    }
}

fn rects_ui(
    ui: &mut egui::Ui,
    markers: &mut SourceMarkers,
    interact_state: &mut InteractState,
    ui_state: &mut UiState,
) {
    if ui.button("Add").clicked() {
        markers.rects.push(RectMarker {
            rect: VideoRect::new(0, 0, 0, 0),
            name: format!("{}", markers.rects.len()),
            color: random_color(),
        });
    }
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut i = 0;
        markers.rects.retain_mut(|marker| {
            let mut retain = true;
            ui.horizontal(|ui| {
                egui::color_picker::color_edit_button_rgb(ui, &mut marker.color);
                if ui_state.rename_index == Some(i) {
                    let re = ui.text_edit_singleline(&mut marker.name);
                    if re.lost_focus() {
                        ui_state.rename_index = None;
                    }
                    re.request_focus();
                } else if ui
                    .selectable_label(ui_state.selected_rect == Some(i), &marker.name)
                    .clicked()
                {
                    ui_state.selected_rect = Some(i);
                }
                if ui.button("🗑").clicked() {
                    if ui_state.selected_rect == Some(i) {
                        ui_state.selected_rect = None;
                    }
                    retain = false;
                }
            });
            i += 1;
            retain
        });
        if let Some(idx) = ui_state.selected_rect {
            ui.separator();
            let marker = &mut markers.rects[idx];
            egui::Grid::new("rects_grid").show(ui, |ui| {
                ui.label("x");
                ui.add(egui::DragValue::new(&mut marker.rect.pos.x));
                ui.label("y");
                ui.add(egui::DragValue::new(&mut marker.rect.pos.y));
                ui.end_row();
                ui.label("w");
                ui.add(egui::DragValue::new(&mut marker.rect.dim.x));
                ui.label("h");
                ui.add(egui::DragValue::new(&mut marker.rect.dim.y));
            });
            if ui
                .add_enabled(
                    interact_state.rect_drag.is_none(),
                    egui::Button::new("select with mouse"),
                )
                .clicked()
            {
                interact_state.rect_drag = Some(RectDrag::new(idx));
            }
            if ui.button("Rename (F2)").clicked() || ui.input(|inp| inp.key_pressed(egui::Key::F2))
            {
                ui_state.rename_index = Some(idx);
            }
        }
    });
}

/// Color that works with egui color picker.
///
/// Conversion from rgb255 messes up because of floating point inaccuracies
pub type EguiFriendlyColor = [f32; 3];

fn random_color() -> EguiFriendlyColor {
    let mut rng = rand::rng();
    [
        rng.random_range(0.1..=1.0),
        rng.random_range(0.1..=1.0),
        rng.random_range(0.1..=1.0),
    ]
}

pub trait EguiFriendlyColorExt {
    fn to_sfml(self) -> Color;
}

impl EguiFriendlyColorExt for EguiFriendlyColor {
    fn to_sfml(self) -> Color {
        let [r, g, b] = self;
        Color::rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }
}
