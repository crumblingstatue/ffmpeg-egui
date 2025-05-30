use {
    super::random_color,
    crate::{
        InteractState, RectDrag, RectMarker, SourceMarkers, TimeSpan, TimespanMarker,
        coords::VideoRect,
        mpv::{
            Mpv,
            properties::{AbLoopA, AbLoopB, TimePos},
        },
        source,
        text::Text,
    },
    egui_sf2g::egui,
};

#[derive(Default)]
pub struct State {
    pub tab: Tab = Tab::Rects,
    selected_timespan: Option<usize>,
    selected_rect: Option<usize>,
    selected_text: Option<usize>,
    rename_index: Option<usize>,
}

pub(super) fn ui(
    ui: &mut egui::Ui,
    ui_state: &mut State,
    source_markers: &mut SourceMarkers,
    interact_state: &mut InteractState,
    src_info: &source::Info,
    texts: &mut Vec<Text>,
    mpv: &Mpv,
) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.tab, Tab::Rects, Tab::Rects.name());
        ui.selectable_value(&mut ui_state.tab, Tab::TimeSpans, Tab::TimeSpans.name());
        ui.selectable_value(&mut ui_state.tab, Tab::Texts, Tab::Texts.name());
    });
    ui.separator();
    match ui_state.tab {
        Tab::Rects => rects_ui(ui, source_markers, interact_state, ui_state),
        Tab::TimeSpans => timespans_ui(ui, source_markers, src_info, ui_state, mpv),
        Tab::Texts => texts_ui(ui, ui_state, texts, src_info, mpv),
    }
}

fn timespans_ui(
    ui: &mut egui::Ui,
    markers: &mut SourceMarkers,
    src_info: &source::Info,
    ui_state: &mut State,
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
        timespan_ui(ui, &mut marker.timespan, src_info, mpv);

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

fn timespan_ui(ui: &mut egui::Ui, timespan: &mut TimeSpan, src_info: &source::Info, mpv: &Mpv) {
    egui::Grid::new("begin_end_grid").show(ui, |ui| {
        ui.label("begin");
        ui.add(egui::DragValue::new(&mut timespan.begin));
        if ui.button("=").on_hover_text("Set to current").clicked() {
            timespan.begin = src_info.time_pos;
        }
        if ui.button("▶").on_hover_text("Seek here").clicked() {
            mpv.set_property::<TimePos>(timespan.begin);
        }
        ui.end_row();
        ui.label("end");
        ui.add(egui::DragValue::new(&mut timespan.end));
        if ui.button("=").on_hover_text("Set to current").clicked() {
            timespan.end = src_info.time_pos;
        }
        if ui.button(">").on_hover_text("Set to end").clicked() {
            timespan.end = src_info.duration;
        }
        if ui.button("▶").on_hover_text("Seek here").clicked() {
            mpv.set_property::<TimePos>(timespan.end);
        }
        ui.end_row();
        ui.label("Duration");
        let dur_s = format!("{:.03}", timespan.end - timespan.begin);
        ui.label(&dur_s);
        if ui.button("copy").clicked() {
            ui.ctx().copy_text(dur_s);
        }
    });
}

fn rects_ui(
    ui: &mut egui::Ui,
    markers: &mut SourceMarkers,
    interact_state: &mut InteractState,
    ui_state: &mut State,
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
    Texts,
}

impl Tab {
    fn name(&self) -> &'static str {
        match self {
            Tab::Rects => "Rects",
            Tab::TimeSpans => "Time spans",
            Tab::Texts => "Texts",
        }
    }
}

fn texts_ui(
    ui: &mut egui::Ui,
    ui_state: &mut State,
    texts: &mut Vec<Text>,
    src_info: &source::Info,
    mpv: &Mpv,
) {
    if ui.button("Add").clicked() {
        texts.push(Text::default());
        ui_state.selected_text = Some(texts.len().saturating_sub(1));
    }
    ui.separator();
    let mut clone_this = None;
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            let mut i = 0;
            texts.retain_mut(|text| {
                let mut retain = true;
                ui.horizontal(|ui| {
                    ui.label(i.to_string());
                    if ui
                        .selectable_label(ui_state.selected_text == Some(i), &text.string)
                        .clicked()
                    {
                        ui_state.selected_text = Some(i);
                    }
                    if ui.button("🗑").clicked() {
                        if ui_state.selected_text == Some(i) {
                            ui_state.selected_text = None;
                        }
                        retain = false;
                    }
                    if ui.button("Dup").clicked() {
                        clone_this = Some(i);
                    }
                });
                i += 1;
                retain
            });
        });
    if let Some(idx) = clone_this {
        texts.push(texts[idx].clone());
        ui_state.selected_text = Some(texts.len().saturating_sub(1));
    }
    ui.separator();
    if let Some(idx) = ui_state.selected_text
        && let Some(text) = texts.get_mut(idx)
    {
        egui::Grid::new("text_grid").num_columns(3).show(ui, |ui| {
            ui.label("Position");
            ui.add(egui::DragValue::new(&mut text.pos.x));
            ui.add(egui::DragValue::new(&mut text.pos.y));
            ui.end_row();
            ui.label("Size");
            ui.add(egui::DragValue::new(&mut text.size));
            ui.end_row();
            ui.label("Border");
            ui.add(egui::DragValue::new(&mut text.borderw));
            ui.end_row();
            ui.label("Font path");
            ui.label("");
            ui.text_edit_singleline(&mut text.font_path);
            ui.end_row();
        });
        ui.text_edit_multiline(&mut text.string);
        ui.separator();
        ui.heading("Timespan");
        timespan_ui(ui, &mut text.timespan, src_info, mpv);
    }
}
