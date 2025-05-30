mod ffmpeg_cli;
pub mod right_panel;

use {
    crate::{
        app::{AppState, load_kashimark_subs_with_opt_timings},
        config::{self, Config},
        coords::{VideoMag, VideoPos},
        mpv::{
            Mpv,
            commands::LoadFile,
            properties::{AudioId, Path, Speed, SubId, TimePos, Volume},
        },
        time_fmt::FfmpegTimeFmt,
    },
    egui_file_dialog::FileDialog,
    egui_sf2g::egui::{self},
    ffmpeg_cli::{FfmpegCli, ffmpeg_cli_ui},
    rand::Rng as _,
};

pub struct UiState {
    pub right_panel: right_panel::State,
    pub ffmpeg_cli: FfmpegCli,
    pub file_dialog: FileDialog,
    pub file_op: FileOp,
    pub modal: ModalPopup,
    pub quit_requested: bool,
}

#[derive(Default)]
pub struct ModalPopup {
    payload: Option<ModalPayload>,
}
impl ModalPopup {
    fn err(&mut self, msg: String) {
        self.payload = Some(ModalPayload::Error { msg })
    }

    fn show(&mut self, ctx: &egui::Context) {
        if let Some(payload) = &self.payload {
            let mut close = false;
            egui::Modal::new("modal_popup".into()).show(ctx, |ui| {
                match payload {
                    ModalPayload::Error { msg } => {
                        ui.heading("Error");
                        ui.label(msg);
                    }
                }
                if ui.button("Close").clicked() {
                    close = true;
                }
            });
            if close {
                self.payload = None;
            }
        }
    }
}

enum ModalPayload {
    Error { msg: String },
}

pub enum FileOp {
    MediaFile,
    Kashimark,
    SubTimings,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            right_panel: right_panel::State::default(),
            ffmpeg_cli: FfmpegCli::default(),
            file_dialog: FileDialog::new().as_modal(true),
            file_op: FileOp::MediaFile,
            modal: ModalPopup::default(),
            quit_requested: false,
        }
    }
}

pub(crate) fn ui(
    ctx: &egui::Context,
    mpv: &mut Mpv,
    app_state: &mut AppState,
    ui_state: &mut UiState,
    cfg: &mut Config,
) {
    let re = egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        bottom_bar_ui(ui, ui_state, mpv, app_state, cfg);
    });
    app_state.video_area_max_dim.y = re.response.rect.top() as VideoMag;
    let re = egui::SidePanel::right("right_panel").show(ctx, |ui| {
        right_panel::ui(
            ui,
            &mut ui_state.right_panel,
            &mut app_state.source_markers,
            &mut app_state.interact,
            &app_state.src,
            &mut app_state.texts,
            mpv,
        );
    });
    app_state.video_area_max_dim.x = re.response.rect.left() as VideoMag;
    if ui_state.ffmpeg_cli.open {
        egui::Window::new("ffmpeg").show(ctx, |ui| {
            if let Some(path) = mpv.get_property::<Path>() {
                app_state.src.path = path.to_owned();
            }
            ffmpeg_cli_ui(
                ui,
                ui_state,
                &app_state.source_markers,
                &app_state.texts,
                &app_state.src,
                cfg,
            );
        });
        ui_state.ffmpeg_cli.first_frame = false;
    }
    ui_state.file_dialog.update(ctx);
    if let Some(path) = ui_state.file_dialog.take_picked() {
        match ui_state.file_op {
            FileOp::MediaFile => {
                cfg.recently_used_list.use_(path.display().to_string());
                mpv.command_async(crate::mpv::commands::LoadFile {
                    path: path.to_str().unwrap(),
                });
            }
            FileOp::Kashimark => match load_kashimark_subs_with_opt_timings(&path, None) {
                Ok(subs) => app_state.subs = Some(subs),
                Err(e) => {
                    ui_state
                        .modal
                        .err(format!("Error loading kashimark subs: {e}"));
                }
            },
            FileOp::SubTimings => {
                if let Some(subs) = &mut app_state.subs
                    && let Err(e) = subs.load_timings(path.display().to_string())
                {
                    ui_state
                        .modal
                        .err(format!("Error loading sub timings: {e}"));
                }
            }
        }
    }
    ui_state.modal.show(ctx);
}

fn bottom_bar_ui(
    ui: &mut egui::Ui,
    ui_state: &mut UiState,
    mpv: &mut Mpv,
    app_state: &mut AppState,
    cfg: &mut Config,
) {
    ui.horizontal(|ui| {
        ui.label(format!(
            "{}/{}",
            FfmpegTimeFmt(app_state.src.time_pos),
            FfmpegTimeFmt(app_state.src.duration)
        ));
        ui.style_mut().spacing.slider_width = ui.available_width();
        let mut pos = app_state.src.time_pos;
        if ui
            .add(egui::Slider::new(&mut pos, 0.0..=app_state.src.duration).show_value(false))
            .changed()
        {
            mpv.set_property::<TimePos>(pos);
        }
    });
    ui.horizontal(|ui| {
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
        ui.menu_button("Menu", |ui| {
            if ui.button("Load media file...").clicked() {
                ui_state.file_dialog.pick_file();
                ui_state.file_op = FileOp::MediaFile;
                ui.close_menu();
            }
            ui.menu_button("Recent", |ui| {
                for item in cfg.recently_used_list.iter() {
                    if ui.button(item).clicked() {
                        ui.close_menu();
                        mpv.command_async(LoadFile { path: item });
                        cfg.recently_used_list.use_(item.clone());
                        return;
                    }
                }
            });
            if ui.button("Load kashimark subs...").clicked() {
                ui_state.file_dialog.pick_file();
                ui_state.file_op = FileOp::Kashimark;
                ui.close_menu();
            }
            if let Some(subs) = &mut app_state.subs {
                if ui.button("↻ Reload kashimark subs").clicked() {
                    ui.close_menu();
                    if let Err(e) = subs.reload() {
                        ui_state.modal.err(format!("Error reloading subs: {e}"));
                    }
                }
                if ui.button("Load sub timings...").clicked() {
                    ui_state.file_dialog.pick_file();
                    ui_state.file_op = FileOp::SubTimings;
                    ui.close_menu();
                }
            }
            if ui.button("Reset pan").clicked() {
                app_state.interact.pan_pos = VideoPos::new(0, 0);
                ui.close_menu();
            }
            ui.menu_button("Video size", |ui| {
                let Some(present) = &mut app_state.present else {
                    return;
                };
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                let mut present_size_changed = false;
                if ui.button("Original").clicked() {
                    present.dim.x = app_state.src.dim.x as VideoMag;
                    present.dim.y = app_state.src.dim.y as VideoMag;
                    present_size_changed = true;
                    ui.close_menu();
                }
                if ui.button("Fit").clicked() {
                    present.dim.y = app_state.video_area_max_dim.y;
                    present.dim.x = (present.dim.y as f64 * app_state.src.w_h_ratio) as VideoMag;
                    if present.dim.x > app_state.video_area_max_dim.x {
                        present.dim.x = app_state.video_area_max_dim.x;
                        present.dim.y =
                            (present.dim.x as f64 / app_state.src.w_h_ratio) as VideoMag;
                    }
                    present_size_changed = true;
                    ui.close_menu();
                }
                ui.label("Width");
                if ui.add(egui::DragValue::new(&mut present.dim.x)).changed() {
                    present.dim.y = (present.dim.x as f64 / app_state.src.w_h_ratio) as VideoMag;
                    present_size_changed = true;
                }
                ui.label("Height");
                if ui.add(egui::DragValue::new(&mut present.dim.y)).changed() {
                    present.dim.x = (present.dim.y as f64 * app_state.src.w_h_ratio) as VideoMag;
                    present_size_changed = true;
                }
                if present_size_changed {
                    // Clamp range to make it somewhat sane
                    dbg!(present.dim);
                    present.dim.x = (present.dim.x).clamp(1, 4096);
                    present.dim.y = (present.dim.y).clamp(1, 4096);
                    dbg!(present.dim);
                    if present
                        .texture
                        .create(
                            (present.dim.x).try_into().unwrap(),
                            (present.dim.y).try_into().unwrap(),
                        )
                        .is_err()
                    {
                        panic!("Failed to create texture");
                    }
                }
            });
            if let Some(mut current) = mpv.get_property::<AudioId>() {
                ui.horizontal(|ui| {
                    ui.label("Audio track");
                    if ui.add(egui::DragValue::new(&mut current)).changed() {
                        mpv.set_property::<AudioId>(current);
                    }
                });
            } else if ui.button("Set audio track to 1").clicked() {
                ui.close_menu();
                mpv.set_property::<AudioId>(1);
            }
            if let Some(mut current) = mpv.get_property::<SubId>() {
                ui.horizontal(|ui| {
                    ui.label("Sub track");
                    if ui.add(egui::DragValue::new(&mut current)).changed() {
                        mpv.set_property::<SubId>(current);
                    }
                });
            } else if ui.button("Set sub track to 1").clicked() {
                ui.close_menu();
                mpv.set_property::<SubId>(1);
            }
            if let Some(subs) = &mut app_state.subs {
                if ui.button("Clear sub timings").clicked() {
                    ui.close_menu();
                    subs.clear();
                }
                if ui.button("Save sub timings to file").clicked() {
                    ui.close_menu();
                    subs.save_timings();
                }
                if let Some(reload) = subs.timings_reload_sentry()
                    && ui.button("↻ Reload sub timings from file").clicked()
                {
                    ui.close_menu();
                    if let Err(e) = reload.reload() {
                        ui_state.modal.err(format!("Error reloading timings: {e}"));
                    }
                }
            }
            ui.separator();
            if ui.button("Open config file").clicked()
                && let Err(e) = config::shell_open()
            {
                ui_state
                    .modal
                    .err(format!("Error opening config file: {e}"));
            }
            ui.separator();
            if ui.button("🚪 Quit").clicked() {
                ui_state.quit_requested = true;
            }
        });
        if mpv.is_idle() {
            ui.label("<mpv idle>");
        }
    });
}

/// Color that works with egui color picker.
///
/// Conversion from rgb255 messes up because of floating point inaccuracies
pub type EguiFriendlyColor = [f32; 3];

fn random_color() -> EguiFriendlyColor {
    let mut rng = rand::rng();
    [
        rng.random_range(0.1..=1.0),
        rng.random_range(0.1..=1.0),
        rng.random_range(0.1..=1.0),
    ]
}
