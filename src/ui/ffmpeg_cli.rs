use {
    super::UiState,
    crate::{SourceMarkers, ffmpeg::resolve_arguments, source},
    egui_sfml::egui,
    std::io::Read as _,
};

#[derive(Default)]
pub struct FfmpegCli {
    pub open: bool,
    pub source_string: String,
    pub first_frame: bool,
    child: Option<std::process::Child>,
    err_str: String,
    exit_status: Option<i32>,
    stdout: String,
    stderr: String,
    cook_book: CookBook,
}

#[derive(Default)]
struct CookBook {
    open: bool,
    recipes: &'static [Recipe] = recipes(),
    selected: Option<usize>,
}

const fn recipes() -> &'static [Recipe] {
    macro_rules! recipes {
        ($($name:literal $($desc:literal)+;)*) => {
            &[
            $(
                Recipe{ name: $name, descriptions: &[$($desc,)+] },
            )*
            ]
        }
    }
    recipes! {
        "Crop video"
        "-vf crop=out_w:out_h:x:y out.mp4";
        "Replace audio track"
        "-i video.mp4 -i audio.wav -c:v copy -map 0:v:0 -map 1:a:0 out.mp4";
        "Burn subtitles"
        "-vf subtitles=subtitle.srt"
        "-vf ass=subtitle.ass out.mp4";
    }
}

struct Recipe {
    name: &'static str,
    descriptions: &'static [&'static str],
}

const FFMPEG_HELP_TEXT: &str = "\
{i}: Currently opened media file
{r.x} Rectangle 'x'
{t.x} Timespan 'x'
";

pub fn ffmpeg_cli_ui(
    ui: &mut egui::Ui,
    ui_state: &mut UiState,
    source_markers: &SourceMarkers,
    src_info: &source::Info,
) {
    if ui_state.ffmpeg_cli.cook_book.open {
        egui::SidePanel::right("cookbook_right_panel").show_inside(ui, |ui| {
            for (i, recipe) in ui_state.ffmpeg_cli.cook_book.recipes.iter().enumerate() {
                if ui
                    .selectable_label(
                        ui_state.ffmpeg_cli.cook_book.selected == Some(i),
                        recipe.name,
                    )
                    .clicked()
                {
                    ui_state.ffmpeg_cli.cook_book.selected = Some(i);
                }
            }
            ui.separator();
            if let Some(sel_idx) = ui_state.ffmpeg_cli.cook_book.selected {
                let recipe = &ui_state.ffmpeg_cli.cook_book.recipes[sel_idx];
                ui.heading(recipe.name);
                for &desc in recipe.descriptions {
                    ui.horizontal(|ui| {
                        if ui.button("🏷").on_hover_text("Copy").clicked() {
                            ui.ctx().copy_text(desc.to_owned());
                        }
                        ui.label(desc);
                    });
                }
            }
        });
    }
    let ctrl_enter = ui.input_mut(|inp| inp.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));
    let re = ui.add(
        egui::TextEdit::multiline(&mut ui_state.ffmpeg_cli.source_string)
            .hint_text("arguments to ffmpeg"),
    );
    match resolve_arguments(&ui_state.ffmpeg_cli.source_string, source_markers, src_info) {
        Ok(args) => {
            let mut args_str = String::new();
            for (i, arg) in args.iter().enumerate() {
                args_str.push_str(&format!("{i}: `{arg}`\n"));
            }
            ui.label(egui::RichText::new(args_str).color(egui::Color32::GOLD));
            if ui.button("run (ctrl+enter)").clicked() || ctrl_enter {
                ui_state.ffmpeg_cli.exit_status = None;
                ui_state.ffmpeg_cli.err_str.clear();
                ui_state.ffmpeg_cli.stderr.clear();
                ui_state.ffmpeg_cli.stdout.clear();
                match crate::ffmpeg::invoke(
                    &ui_state.ffmpeg_cli.source_string,
                    source_markers,
                    src_info,
                ) {
                    Ok(child) => ui_state.ffmpeg_cli.child = Some(child),
                    Err(e) => ui_state.ffmpeg_cli.err_str = e.to_string(),
                }
            }
        }
        Err(e) => {
            ui.label(egui::RichText::new(e.to_string()).color(egui::Color32::RED));
        }
    }
    if ui_state.ffmpeg_cli.first_frame {
        re.request_focus();
    }
    ui.label(FFMPEG_HELP_TEXT);
    if !ui_state.ffmpeg_cli.err_str.is_empty() {
        ui.label(egui::RichText::new(&ui_state.ffmpeg_cli.err_str).color(egui::Color32::RED));
    }
    if let Some(child) = &mut ui_state.ffmpeg_cli.child {
        ui.horizontal(|ui| {
            ui.label("running ffmpeg");
            if ui.button("kill").clicked() {
                if let Err(e) = child.kill() {
                    eprintln!("Error killing child process: {e}");
                }
            }
            ui.spinner();
        });
        match child.try_wait() {
            Ok(Some(status)) => {
                ui_state.ffmpeg_cli.exit_status = status.code();
                if let Some(mut stdout) = child.stdout.take() {
                    let mut buf = Vec::new();
                    stdout.read_to_end(&mut buf).unwrap();
                    ui_state.ffmpeg_cli.stdout = String::from_utf8_lossy(&buf).into_owned();
                }
                if let Some(mut stderr) = child.stderr.take() {
                    let mut buf = Vec::new();
                    stderr.read_to_end(&mut buf).unwrap();
                    ui_state.ffmpeg_cli.stderr = String::from_utf8_lossy(&buf).into_owned();
                }
                ui_state.ffmpeg_cli.child = None;
            }
            Ok(None) => {}
            Err(e) => ui_state.ffmpeg_cli.err_str = e.to_string(),
        }
    }
    if let Some(code) = ui_state.ffmpeg_cli.exit_status {
        ui.label(format!("Exit status: {}", code));
    }
    if !ui_state.ffmpeg_cli.stdout.is_empty() {
        ui.label("Standard output:");
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .id_salt("stdout")
            .show(ui, |ui| {
                ui.text_edit_multiline(&mut ui_state.ffmpeg_cli.stdout);
            });
    }
    if !ui_state.ffmpeg_cli.stderr.is_empty() {
        ui.label("Standard error:");
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .id_salt("stderr")
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.text_edit_multiline(&mut ui_state.ffmpeg_cli.stderr);
            });
    }
    ui.checkbox(&mut ui_state.ffmpeg_cli.cook_book.open, "Cook book");
}
