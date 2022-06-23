use egui_sfml::egui;
use sfml::graphics::Rect;

use crate::{
    coords::VideoRect,
    mpv::{
        properties::{Speed, TimePos, Volume},
        Mpv,
    },
    present::Present,
    source,
    time_fmt::FfmpegTimeFmt,
};

pub(crate) fn ui(
    ctx: &egui::Context,
    mpv: &mut Mpv,
    video_area_max_h: &mut f32,
    present: &mut Present,
    rects: &mut Vec<VideoRect>,
    src_info: &source::Info,
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
                if ui
                    .add(egui::DragValue::new(&mut present.dim.width))
                    .changed()
                {
                    present.dim.height = (present.dim.width as f64 / src_info.w_h_ratio) as u16;
                    changed = true;
                }
                ui.label("Video height");
                if ui
                    .add(egui::DragValue::new(&mut present.dim.height))
                    .changed()
                {
                    present.dim.width = (present.dim.height as f64 * src_info.w_h_ratio) as u16;
                    changed = true;
                }
                if ui.button("orig").clicked() {
                    present.dim.width = src_info.dim.width as u16;
                    present.dim.height = src_info.dim.height as u16;
                    changed = true;
                }
                if ui.button("fit").clicked() {
                    present.dim.height = *video_area_max_h as u16;
                    present.dim.width = (present.dim.height as f64 * src_info.w_h_ratio) as u16;
                    changed = true;
                }
                // Clamp range to make it somewhat sane
                present.dim.width = (present.dim.width).clamp(1, 4096);
                present.dim.height = (present.dim.height).clamp(1, 4096);
                if changed
                    && !present
                        .texture
                        .create((present.dim.width).into(), (present.dim.height).into())
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
                rects.push(Rect::default());
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.separator();
                for rect in rects {
                    ui.label("left");
                    ui.add(egui::DragValue::new(&mut rect.left));
                    ui.label("top");
                    ui.add(egui::DragValue::new(&mut rect.top));
                    ui.label("w");
                    ui.add(egui::DragValue::new(&mut rect.width));
                    ui.label("h");
                    ui.add(egui::DragValue::new(&mut rect.height));
                    ui.separator();
                }
            });
        });
    }
}
