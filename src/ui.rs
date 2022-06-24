use egui_sfml::egui::{self, ScrollArea};
use rand::{thread_rng, Rng};
use sfml::graphics::Color;

use crate::{
    coords::{self, VideoDim, VideoMag, VideoRect},
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
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            tab: Tab::Rects,
            selected_timespan: None,
            rename_index: None,
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
    mpv: &mut Mpv,
    video_area_max_dim: &mut VideoDim<coords::Present>,
    present: &mut Present,
    source_markers: &mut SourceMarkers,
    src_info: &source::Info,
    interact_state: &mut InteractState,
    ui_state: &mut UiState,
) {
    {
        let re = egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            bottom_bar_ui(ui, src_info, present, mpv, video_area_max_dim);
        });
        video_area_max_dim.y = re.response.rect.top() as VideoMag;
        let re = egui::SidePanel::right("right_panel").show(ctx, |ui| {
            right_panel_ui(ui, ui_state, source_markers, interact_state, src_info, mpv);
        });
        video_area_max_dim.x = re.response.rect.left() as VideoMag;
    }
}

fn right_panel_ui(
    ui: &mut egui::Ui,
    ui_state: &mut UiState,
    source_markers: &mut SourceMarkers,
    interact_state: &mut InteractState,
    src_info: &source::Info,
    mpv: &mut Mpv,
) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.tab, Tab::Rects, Tab::Rects.name());
        ui.selectable_value(&mut ui_state.tab, Tab::TimeSpans, Tab::TimeSpans.name());
    });
    ui.separator();
    match ui_state.tab {
        Tab::Rects => rects_ui(ui, source_markers, interact_state),
        Tab::TimeSpans => timespans_ui(ui, source_markers, src_info, ui_state, mpv),
    }
}

fn bottom_bar_ui(
    ui: &mut egui::Ui,
    src_info: &source::Info,
    present: &mut Present,
    mpv: &mut Mpv,
    video_area_max_dim: &mut VideoDim<coords::Present>,
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
    });
}

fn timespans_ui(
    ui: &mut egui::Ui,
    markers: &mut SourceMarkers,
    src_info: &source::Info,
    ui_state: &mut UiState,
    mpv: &mut Mpv,
) {
    if ui.button("Add").clicked() {
        markers.timespans.push(TimespanMarker {
            timespan: TimeSpan {
                begin: src_info.time_pos,
                end: src_info.time_pos,
            },
            name: format!("Timespan {}", markers.timespans.len()),
            color: random_color(),
        });
    }
    ui.separator();
    ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
        for (i, marker) in markers.timespans.iter_mut().enumerate() {
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
            });
        }
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
            if ui.button("▶").on_hover_text("Seek here").clicked() {
                mpv.set_property::<TimePos>(marker.timespan.end);
            }
            ui.end_row();
        });

        if ui.button("Rename (F2)").clicked() || ui.input().key_pressed(egui::Key::F2) {
            ui_state.rename_index = Some(timespan_idx);
        }
        if ui.button("A-B loop").clicked() {
            mpv.set_property::<AbLoopA>(marker.timespan.begin);
            mpv.set_property::<AbLoopB>(marker.timespan.end);
        }
    }
    ui.separator();
    if ui.button("Clear A-B loop").clicked() {
        todo!()
    }
}

fn rects_ui(ui: &mut egui::Ui, markers: &mut SourceMarkers, interact_state: &mut InteractState) {
    if ui.button("Add").clicked() {
        markers.rects.push(RectMarker {
            rect: VideoRect::new(0, 0, 0, 0),
            color: random_color(),
        });
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.separator();
        for (i, marker) in markers.rects.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label("x");
                ui.add(egui::DragValue::new(&mut marker.rect.pos.x));
                ui.label("y");
                ui.add(egui::DragValue::new(&mut marker.rect.pos.y));
            });
            ui.horizontal(|ui| {
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
                interact_state.rect_drag = Some(RectDrag::new(i));
            }

            egui::color_picker::color_edit_button_rgb(ui, &mut marker.color);
            ui.separator();
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
