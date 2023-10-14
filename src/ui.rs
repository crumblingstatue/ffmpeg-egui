use std::io::Read;

use egui_sfml::egui::{self, RichText, ScrollArea};
use rand::{thread_rng, Rng};
use sfml::graphics::Color;

use crate::{
    coords::{self, VideoDim, VideoMag, VideoRect},
    ffmpeg,
    mpv::{
        properties::{AbLoopA, AbLoopB, Speed, TimePos, Volume},
        Mpv,
    },
    present::Present,
    source,
    time_fmt::FfmpegTimeFmt,
    InteractState, RectDrag, RectMarker, SourceMarkers, TimeSpan, TimespanMarker,
};

pub struct UiState {
    tab: Tab,
    selected_timespan: Option<usize>,
    rename_index: Option<usize>,
    selected_rect: Option<usize>,
    ffmpeg_cli: FfmpegCli,
}

#[derive(Default)]
struct FfmpegCli {
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
        }
    }
}

#[derive(PartialEq, Eq)]
enum Tab {
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
    mpv: &Mpv,
    video_area_max_dim: &mut VideoDim<coords::Present>,
    present: &mut Present,
    source_markers: &mut SourceMarkers,
    src_info: &source::Info,
    interact_state: &mut InteractState,
    ui_state: &mut UiState,
) {
    {
        let re = egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            bottom_bar_ui(ui, src_info, present, mpv, video_area_max_dim, ui_state);
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
    }
}

fn ffmpeg_cli_ui(
    ui: &mut egui::Ui,
    ui_state: &mut UiState,
    source_markers: &SourceMarkers,
    src_info: &source::Info,
) {
    ui.label("ffmpeg");
    let ctrl_enter = ui.input_mut(|inp| inp.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));
    let re = ui.text_edit_multiline(&mut ui_state.ffmpeg_cli.source_string);
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
    if ui_state.ffmpeg_cli.first_frame {
        re.request_focus();
    }
    ui.label("help: {input}, {rect}, {t.x}");
    if !ui_state.ffmpeg_cli.err_str.is_empty() {
        ui.label(RichText::new(&ui_state.ffmpeg_cli.err_str).color(egui::Color32::RED));
    }
    if let Some(child) = &mut ui_state.ffmpeg_cli.child {
        ui.horizontal(|ui| {
            ui.label("running ffmpeg");
            if ui.button("kill").clicked() {
                if let Err(e) = child.kill() {
                    rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Error)
                        .set_title("Process kill error")
                        .set_description(e.to_string())
                        .show();
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
        ScrollArea::vertical().show(ui, |ui| {
            ui.text_edit_multiline(&mut ui_state.ffmpeg_cli.stdout);
        });
    }
    if !ui_state.ffmpeg_cli.stderr.is_empty() {
        ui.label("Standard error:");
        ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
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

fn bottom_bar_ui(
    ui: &mut egui::Ui,
    src_info: &source::Info,
    present: &mut Present,
    mpv: &Mpv,
    video_area_max_dim: &VideoDim<coords::Present>,
    ui_state: &mut UiState,
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
        let mut changed = false;
        ui.label("Video width");
        if ui.add(egui::DragValue::new(&mut present.dim.x)).changed() {
            present.dim.y = (present.dim.x as f64 / src_info.w_h_ratio) as VideoMag;
            changed = true;
        }
        ui.label("Video height");
        if ui.add(egui::DragValue::new(&mut present.dim.y)).changed() {
            present.dim.x = (present.dim.y as f64 * src_info.w_h_ratio) as VideoMag;
            changed = true;
        }
        if ui.button("orig").clicked() {
            present.dim.x = src_info.dim.x as VideoMag;
            present.dim.y = src_info.dim.y as VideoMag;
            changed = true;
        }
        if ui.button("fit").clicked() {
            present.dim.y = video_area_max_dim.y;
            present.dim.x = (present.dim.y as f64 * src_info.w_h_ratio) as VideoMag;
            if present.dim.x > video_area_max_dim.x {
                present.dim.x = video_area_max_dim.x;
                present.dim.y = (present.dim.x as f64 / src_info.w_h_ratio) as VideoMag;
            }
            changed = true;
        }
        // Clamp range to make it somewhat sane
        present.dim.x = (present.dim.x).clamp(1, 4096);
        present.dim.y = (present.dim.y).clamp(1, 4096);
        if changed
            && !present
                .texture
                .create((present.dim.x).into(), (present.dim.y).into())
        {
            panic!("Failed to create texture");
        }
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
                ui.output_mut(|o| o.copied_text = dur_s);
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
    let mut rng = thread_rng();
    [
        rng.gen_range(0.1..=1.0),
        rng.gen_range(0.1..=1.0),
        rng.gen_range(0.1..=1.0),
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
