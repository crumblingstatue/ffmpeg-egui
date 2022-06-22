#![feature(array_chunks)]
#![feature(generic_const_exprs, generic_associated_types)]

mod mpv;
mod time_fmt;

use egui_sfml::{egui, SfEgui};
use std::fmt::Write;
use time_fmt::FfmpegTimeFmt;

use mpv::{
    commands::{FrameBackStep, FrameStep, LoadFile, PlaylistPlay},
    properties::{
        AudioPitchCorrection, Duration, Height, KeepOpen, KeepOpenPause, Pause, Speed, TimePos,
        Volume, Width,
    },
    property::{YesNo, YesNoAlways},
    Mpv,
};
use sfml::{
    graphics::{
        Color, Font, Rect, RectangleShape, RenderTarget, RenderWindow, Shape, Sprite, Text,
        Texture, Transformable, View,
    },
    window::{ContextSettings, Event, Key, Style},
};

fn main() {
    let path = std::env::args().nth(1).expect("Need path to media file");
    let mut mpv = Mpv::new().unwrap();
    mpv.set_property::<AudioPitchCorrection>(false);
    mpv.set_property::<KeepOpen>(YesNoAlways::Yes);
    mpv.set_property::<KeepOpenPause>(YesNo::No);
    mpv.command_async(LoadFile { path: &path });
    let mut rects: Vec<Rect<u16>> = Vec::new();
    let mut rw = RenderWindow::new(
        (960, 600),
        "ffmpeg-egui",
        Style::RESIZE,
        &ContextSettings::default(),
    );
    rw.set_framerate_limit(60);
    let mut sf_egui = SfEgui::new(&rw);

    let mut tex = Texture::new().unwrap();
    let mut video_w: u16 = 800;
    let mut video_h: u16 = 600;
    if !tex.create(video_w.into(), video_h.into()) {
        panic!("Failed to create texture");
    }

    let font = unsafe { Font::from_memory(include_bytes!("../DejaVuSansMono.ttf")).unwrap() };
    let prefix = "Mouse video pos: ";
    let mut pos_string = String::from(prefix);
    let mut overlay_show = true;
    let actual_video_w = mpv.get_property::<Width>().unwrap();
    let actual_video_h = mpv.get_property::<Height>().unwrap();
    let w_h_ratio = actual_video_w as f64 / actual_video_h as f64;
    let mut video_area_max_h = 100.0;

    while rw.is_open() {
        while let Some(event) = rw.poll_event() {
            sf_egui.add_event(&event);
            match event {
                Event::Closed => rw.close(),
                Event::KeyPressed { code, .. } => match code {
                    Key::Escape => rw.close(),
                    Key::Tab => overlay_show ^= true,
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
                    Key::P => mpv.command_async(PlaylistPlay::Current),
                    Key::S => mpv.command_async(PlaylistPlay::None),
                    Key::R => mpv.command_async(PlaylistPlay::Index(0)),
                    _ => {}
                },
                Event::Resized { width, height } => {
                    let view = View::from_rect(&Rect::new(0., 0., width as f32, height as f32));
                    rw.set_view(&view);
                }
                _ => {}
            }
        }
        let mouse_pos = rw.mouse_position();
        let duration = mpv.get_property::<Duration>().unwrap_or(0.0);
        sf_egui.do_frame(|ctx| {
            let re = egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                if let Some(mut pos) = mpv.get_property::<TimePos>() {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{}/{}",
                            FfmpegTimeFmt(pos),
                            FfmpegTimeFmt(duration)
                        ));
                        ui.style_mut().spacing.slider_width = ui.available_width();
                        if ui
                            .add(egui::Slider::new(&mut pos, 0.0..=duration).show_value(false))
                            .changed()
                        {
                            mpv.set_property::<TimePos>(pos);
                        }
                    });
                }
                ui.horizontal(|ui| {
                    let mut changed = false;
                    ui.label("Video width");
                    if ui.add(egui::DragValue::new(&mut video_w)).changed() {
                        video_h = (video_w as f64 / w_h_ratio) as u16;
                        changed = true;
                    }
                    ui.label("Video height");
                    if ui.add(egui::DragValue::new(&mut video_h)).changed() {
                        video_w = (video_h as f64 * w_h_ratio) as u16;
                        changed = true;
                    }
                    if ui.button("orig").clicked() {
                        video_w = actual_video_w as u16;
                        video_h = actual_video_h as u16;
                        changed = true;
                    }
                    if ui.button("fit").clicked() {
                        video_h = video_area_max_h as u16;
                        video_w = (video_h as f64 * w_h_ratio) as u16;
                        changed = true;
                    }
                    // Clamp range to make it somewhat sane
                    video_w = video_w.clamp(1, 4096);
                    video_h = video_h.clamp(1, 4096);
                    if changed && !tex.create(video_w.into(), video_h.into()) {
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
            video_area_max_h = re.response.rect.top();
            egui::SidePanel::right("right_panel").show(ctx, |ui| {
                if ui.button("Add rect").clicked() {
                    rects.push(Rect::default());
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.separator();
                    for rect in &mut rects {
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
        });
        let (mvx, mvy) =
            video_mouse_pos(mouse_pos, actual_video_w, actual_video_h, video_w, video_h);
        pos_string.truncate(prefix.len());
        write!(&mut pos_string, "{}, {}", mvx, mvy,).unwrap();
        rw.clear(Color::BLACK);

        unsafe {
            let pixels = mpv.get_frame_as_pixels(video_w, video_h);
            tex.update_from_pixels(pixels, video_w.into(), video_h.into(), 0, 0);
        }
        rw.draw(&Sprite::with_texture(&tex));
        if overlay_show {
            rw.draw(&Text::new(&pos_string, &font, 32));
            let mut rs = RectangleShape::default();
            rs.set_fill_color(Color::rgba(250, 250, 200, 128));
            for rect in &rects {
                let (w, h) = translate_up(
                    rect.width as i32,
                    rect.height as i32,
                    actual_video_w,
                    actual_video_h,
                    video_w,
                    video_h,
                );
                rs.set_size((w as f32, h as f32));
                let (x, y) = translate_up(
                    rect.left as i32,
                    rect.top as i32,
                    actual_video_w,
                    actual_video_h,
                    video_w,
                    video_h,
                );
                rs.set_position((x as f32, y as f32));
                rw.draw(&rs);
            }
        }
        sf_egui.draw(&mut rw, None);
        rw.display();
    }
}

fn video_mouse_pos(
    mouse_pos: sfml::system::Vector2<i32>,
    actual_video_w: i64,
    actual_video_h: i64,
    video_w: u16,
    video_h: u16,
) -> (i16, i16) {
    translate_down(
        mouse_pos.x,
        mouse_pos.y,
        actual_video_w,
        actual_video_h,
        video_w,
        video_h,
    )
}

/// window -> vid coords
fn translate_down(
    x: i32,
    y: i32,
    actual_video_w: i64,
    actual_video_h: i64,
    video_w: u16,
    video_h: u16,
) -> (i16, i16) {
    let w_ratio = actual_video_w as f64 / video_w as f64;
    let h_ratio = actual_video_h as f64 / video_h as f64;
    ((x as f64 * w_ratio) as i16, (y as f64 * h_ratio) as i16)
}

/// vid -> window coords
fn translate_up(
    x: i32,
    y: i32,
    actual_video_w: i64,
    actual_video_h: i64,
    video_w: u16,
    video_h: u16,
) -> (i16, i16) {
    let w_ratio = video_w as f64 / actual_video_w as f64;
    let h_ratio = video_h as f64 / actual_video_h as f64;
    ((x as f64 * w_ratio) as i16, (y as f64 * h_ratio) as i16)
}

fn video_pix_size(w: u16, h: u16) -> usize {
    (w as usize * h as usize) * 4
}
