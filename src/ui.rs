use egui_sfml::egui;

use crate::{
    coords::{VideoMag, VideoRect},
    mpv::{
        properties::{Speed, TimePos, Volume},
        Mpv,
    },
    present::Present,
    source,
    time_fmt::FfmpegTimeFmt,
    InteractState, RectDrag,
};

pub(crate) fn ui(
    ctx: &egui::Context,
    mpv: &mut Mpv,
    video_area_max_h: &mut f32,
    present: &mut Present,
    rects: &mut Vec<VideoRect>,
    src_info: &source::Info,
    interact_state: &mut InteractState,
) {
    {
        let re = egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            if let Some(mut pos) = mpv.get_property::<TimePos>() {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{}/{}",
                        FfmpegTimeFmt(pos),
                        FfmpegTimeFmt(src_info.duration)
                    ));
                    ui.style_mut().spacing.slider_width = ui.available_width();
                    if ui
                        .add(egui::Slider::new(&mut pos, 0.0..=src_info.duration).show_value(false))
                        .changed()
                    {
                        mpv.set_property::<TimePos>(pos);
                    }
                });
            }
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
                    present.dim.y = *video_area_max_h as VideoMag;
                    present.dim.x = (present.dim.y as f64 * src_info.w_h_ratio) as VideoMag;
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
        });
        *video_area_max_h = re.response.rect.top();
        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            if ui.button("Add rect").clicked() {
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
        });
    }
}
