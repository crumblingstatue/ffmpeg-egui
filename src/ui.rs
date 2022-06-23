use egui_sfml::egui;

use crate::{
    coords::{self, Src, VideoDim, VideoMag, VideoRect},
    mpv::{
        properties::{Speed, TimePos, Volume},
        Mpv,
    },
    present::Present,
    source,
    time_fmt::FfmpegTimeFmt,
    InteractState, RectDrag, SourceMarkers,
};

pub struct UiState {
    tab: Tab,
}

impl Default for UiState {
    fn default() -> Self {
        Self { tab: Tab::Rects }
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
            right_panel_ui(ui, ui_state, source_markers, interact_state);
        });
        video_area_max_dim.x = re.response.rect.left() as VideoMag;
    }
}

fn right_panel_ui(
    ui: &mut egui::Ui,
    ui_state: &mut UiState,
    source_markers: &mut SourceMarkers,
    interact_state: &mut InteractState,
) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.tab, Tab::Rects, Tab::Rects.name());
        ui.selectable_value(&mut ui_state.tab, Tab::TimeSpans, Tab::TimeSpans.name());
    });
    ui.separator();
    match ui_state.tab {
        Tab::Rects => rects_ui(ui, &mut source_markers.rects, interact_state),
        Tab::TimeSpans => timespans_ui(ui),
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

fn timespans_ui(ui: &mut egui::Ui) {
    ui.label("Time spans ui");
}

fn rects_ui(
    ui: &mut egui::Ui,
    rects: &mut Vec<VideoRect<Src>>,
    interact_state: &mut InteractState,
) {
    if ui.button("Add").clicked() {
        rects.push(VideoRect::new(0, 0, 0, 0));
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.separator();
        for (i, rect) in rects.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label("x");
                ui.add(egui::DragValue::new(&mut rect.pos.x));
                ui.label("y");
                ui.add(egui::DragValue::new(&mut rect.pos.y));
            });
            ui.horizontal(|ui| {
                ui.label("w");
                ui.add(egui::DragValue::new(&mut rect.dim.x));
                ui.label("h");
                ui.add(egui::DragValue::new(&mut rect.dim.y));
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
            ui.separator();
        }
    });
}
