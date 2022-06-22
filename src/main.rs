#![feature(array_chunks)]
#![feature(generic_const_exprs, generic_associated_types)]

mod mpv;

use egui_sfml::{egui, SfEgui};
use std::fmt::{self, Write};

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
    graphics::{Color, Font, Rect, RenderTarget, RenderWindow, Sprite, Text, Texture, View},
    window::{ContextSettings, Event, Key, Style},
};

fn main() {
    let path = std::env::args().nth(1).expect("Need path to media file");
    let mut mpv = Mpv::new().unwrap();
    mpv.set_property::<AudioPitchCorrection>(false);
    mpv.set_property::<KeepOpen>(YesNoAlways::Yes);
    mpv.set_property::<KeepOpenPause>(YesNo::No);
    mpv.command_async(LoadFile { path: &path });
    let mut rw = RenderWindow::new(
        (800, 600),
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
    let prefix = "SFML Overlay: ";
    let mut pos_string = String::from(prefix);
    let mut overlay_show = true;

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
        let duration = mpv.get_property::<Duration>().unwrap_or(0.0);
        sf_egui.do_frame(|ctx| {
            egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
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
                    changed |= ui.add(egui::DragValue::new(&mut video_w)).changed();
                    ui.label("Video height");
                    changed |= ui.add(egui::DragValue::new(&mut video_h)).changed();
                    if ui.button("1:1").clicked() {
                        let w = mpv.get_property::<Width>().unwrap();
                        let h = mpv.get_property::<Height>().unwrap();
                        video_w = w as u16;
                        video_h = h as u16;
                        changed = true;
                    }
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
        });
        if let Some(pos) = mpv.get_property::<TimePos>() {
            pos_string.truncate(prefix.len());
            write!(
                &mut pos_string,
                "{}/{}",
                FfmpegTimeFmt(pos),
                FfmpegTimeFmt(duration)
            )
            .unwrap();
        }
        rw.clear(Color::BLACK);

        unsafe {
            let pixels = mpv.get_frame_as_pixels(video_w, video_h);
            tex.update_from_pixels(pixels, video_w.into(), video_h.into(), 0, 0);
        }
        rw.draw(&Sprite::with_texture(&tex));
        if overlay_show {
            rw.draw(&Text::new(&pos_string, &font, 32));
        }
        sf_egui.draw(&mut rw, None);
        rw.display();
    }
}

fn video_pix_size(w: u16, h: u16) -> usize {
    (w as usize * h as usize) * 4
}

struct FfmpegTimeFmt(f64);

impl fmt::Display for FfmpegTimeFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.0;
        let hh = secs / 3600.0;
        let mm = hh.fract() * 60.0;
        let ss = mm.fract() * 60.0;
        write!(
            f,
            "{:02.0}:{:02.0}:{:02.0}.{:03}",
            hh.floor(),
            mm.floor(),
            ss.floor(),
            (ss.fract() * 1000.0).round() as u64
        )
    }
}

#[test]
fn test_time_fmt() {
    assert_eq!(&FfmpegTimeFmt(0.0).to_string()[..], "00:00:00.000");
    assert_eq!(&FfmpegTimeFmt(24.56).to_string()[..], "00:00:24.560");
    assert_eq!(&FfmpegTimeFmt(119.885).to_string()[..], "00:01:59.885");
    assert_eq!(&FfmpegTimeFmt(52349.345).to_string()[..], "14:32:29.345");
}
