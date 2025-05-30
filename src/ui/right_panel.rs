use {
    super::{UiState, random_color},
    crate::{
        InteractState, RectDrag, RectMarker, SourceMarkers, TimeSpan, TimespanMarker,
        coords::VideoRect,
        mpv::{
            Mpv,
            properties::{AbLoopA, AbLoopB, TimePos},
        },
        source,
    },
    egui_sf2g::egui,
};

pub(super) fn ui(
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
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
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
                        let re = ui
                            .selectable_label(ui_state.selected_timespan == Some(i), &marker.name);
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
            format!("ab-loop: {a}-{b}")
        }
        (Some(a), None) => {
            format!("loop from {a}")
        }
        (None, Some(b)) => {
            format!("loop to {b}")
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
