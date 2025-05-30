#![feature(
    array_chunks,
    generic_const_exprs,
    let_chains,
    array_windows,
    default_field_values
)]
// We make light use of generic_const_exprs, which is an incomplete feature
#![expect(incomplete_features)]

use {
    crate::mpv::properties::{CropH, CropW, CropY, Rotate},
    app::App,
    clap::Parser,
    config::Config,
    coords::{Src, VideoPos, VideoRect},
    egui_sf2g::sf2g::graphics::Font,
    mpv::{
        commands::LoadFile,
        properties::{AudioPitchCorrection, CropX, Height, KeepOpen, KeepOpenPause, Volume, Width},
        property::{YesNo, YesNoAlways},
    },
    ui::EguiFriendlyColor,
};

mod app;
mod config;
mod coords;
mod ffmpeg;
mod mpv;
mod overlay;
mod present;
mod sfml_integ;
mod source;
mod subs;
mod text;
mod time_fmt;
mod ui;

struct RectDrag {
    idx: usize,
    status: RectDragStatus,
}

struct RectMarker {
    rect: VideoRect<Src>,
    name: String,
    color: EguiFriendlyColor,
}

struct TimespanMarker {
    timespan: TimeSpan,
    name: String,
    color: EguiFriendlyColor,
}

#[derive(Default)]
struct SourceMarkers {
    rects: Vec<RectMarker>,
    timespans: Vec<TimespanMarker>,
}

impl RectDrag {
    fn new(idx: usize) -> Self {
        Self {
            idx,
            status: RectDragStatus::Init,
        }
    }
}

enum RectDragStatus {
    Init,
    ClickedTopLeft,
}

struct InteractState {
    rect_drag: Option<RectDrag>,
    pan_cursor_origin: Option<VideoPos<Src>>,
    pan_image_original_pos: Option<VideoPos<Src>>,
    pan_pos: VideoPos<Src>,
}

impl Default for InteractState {
    fn default() -> Self {
        Self {
            rect_drag: Default::default(),
            pan_cursor_origin: Default::default(),
            pan_image_original_pos: Default::default(),
            pan_pos: VideoPos::new(0, 0),
        }
    }
}

#[derive(Clone, Copy)]
pub struct TimeSpan {
    pub begin: f64,
    pub end: f64,
}
impl TimeSpan {
    pub fn contains(&self, pos: f64) -> bool {
        (self.begin..self.end).contains(&pos)
    }
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum TabOpen {
    Rects,
    Timespans,
}

#[derive(clap::Parser)]
struct Args {
    /// File to open. File picker will open if not supplied.
    file: Option<String>,
    /// Preset the contents of the FFmpeg CLI input
    #[arg(long)]
    ffmpeg_preset: Option<String>,
    /// Start with FFmpeg CLI window open
    #[arg(long)]
    open_cli_win: bool,
    /// Start with a tab open
    #[arg(long)]
    tab: Option<TabOpen>,
    /// Optional kashimark subtitle file to sync against lyrics
    #[arg(long)]
    kashimark: Option<String>,
    /// Optional timing file for subtitle
    #[arg(long)]
    kashimark_timing: Option<String>,
    /// Path to optional overlay font to use instead of default
    #[arg(long)]
    font: Option<String>,
    /// Generate ASS subtitles from opened lyrics and timing, then exit
    #[arg(long)]
    gen_ass: Option<String>,
    /// Use most recently opened file, if any
    #[arg(long)]
    recent: bool,
}

const MOUSE_OVERLAY_PREFIX: &str = "Mouse video pos: ";

fn main() {
    let args = Args::parse();
    let cfg = Config::load_or_default();
    let mut app = App::new(&args, cfg);
    app.mpv.set_property::<AudioPitchCorrection>(false);
    app.mpv.set_property::<KeepOpen>(YesNoAlways::Yes);
    app.mpv.set_property::<KeepOpenPause>(YesNo::No);
    app.mpv.set_property::<Volume>(75.0);
    if let Some(path) = &args.file {
        app.cfg.recently_used_list.use_(path.clone());
        app.mpv.command_async(LoadFile { path });
    } else if args.recent
        && let Some(path) = app.cfg.recently_used_list.most_recent()
    {
        app.mpv.command_async(LoadFile { path });
    }
    app.rw.set_framerate_limit(60);

    let font = match args.font {
        Some(path) => Font::from_file(&path).unwrap(),
        None => Font::from_memory_static(include_bytes!("../DejaVuSansMono.ttf")).unwrap(),
    };
    let actual_video_w = app.mpv.get_property::<Width>().unwrap_or(0);
    let actual_video_h = app.mpv.get_property::<Height>().unwrap_or(0);
    if let Some(ref mut subs) = app.state.subs
        && let Some(path) = args.gen_ass
    {
        subs.write_ass(&path, actual_video_w, actual_video_h);
        return;
    }
    let crop_x = app.mpv.get_property::<CropX>().unwrap_or(0);
    let crop_y = app.mpv.get_property::<CropY>().unwrap_or(0);
    let crop_w = app.mpv.get_property::<CropW>().unwrap_or(0);
    let crop_h = app.mpv.get_property::<CropH>().unwrap_or(0);
    let rotate = app.mpv.get_property::<Rotate>().unwrap_or(0);
    dbg!(crop_x, crop_y, crop_w, crop_h, rotate);
    if rotate != 0 {
        eprintln!("Rotated videos are currently unsupported");
        return;
    }

    while app.rw.is_open() {
        app.do_frame(&font);
    }

    app.save_cfg();
}
