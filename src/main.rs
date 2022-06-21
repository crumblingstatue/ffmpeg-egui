#![feature(array_chunks)]

mod mpv;

use egui_sfml::{egui, SfEgui};

use mpv::{Command, Mpv};
use sfml::{
    graphics::{Color, Font, Rect, RenderTarget, RenderWindow, Sprite, Text, Texture, View},
    window::{ContextSettings, Event, Key, Style},
};

fn main() {
    let path = std::env::args().nth(1).expect("Need path to media file");
    let mut mpv = Mpv::new().unwrap();
    mpv.command_async(Command::LoadFile { path: &path });
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
                    _ => {}
                },
                Event::Resized { width, height } => {
                    let view = View::from_rect(&Rect::new(0., 0., width as f32, height as f32));
                    rw.set_view(&view);
                }
                _ => {}
            }
        }
        sf_egui.do_frame(|ctx| {
            egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let mut changed = false;
                    ui.label("Video width");
                    changed |= ui.add(egui::DragValue::new(&mut video_w)).changed();
                    ui.label("Video height");
                    changed |= ui.add(egui::DragValue::new(&mut video_h)).changed();
                    if changed && !tex.create(video_w.into(), video_h.into()) {
                        panic!("Failed to create texture");
                    }
                });
            });
        });
        if let Some(pos) = mpv.get_property_string("time-pos") {
            pos_string.truncate(prefix.len());
            pos_string.push_str(&pos);
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
