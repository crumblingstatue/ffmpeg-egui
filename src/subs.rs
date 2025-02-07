use {
    egui_sfml::egui::{TextBuffer, ahash::HashMap},
    std::{collections::VecDeque, fmt::Write as _},
};

pub struct SubsState {
    lines: Vec<kashimark::Line>,
    pub tracking: TrackingState,
    saved: Save,
    timings_path: Option<String>,
    pub time_stamps: Vec<f64>,
}

#[derive(Default)]
struct Save {
    tracking: TrackingState,
    time_stamps: Vec<f64>,
}

impl SubsState {
    pub fn new(lines: Vec<kashimark::Line>) -> Self {
        Self {
            lines,
            tracking: TrackingState::default(),
            saved: Save::default(),
            time_stamps: Vec::new(),
            timings_path: None,
        }
    }
    pub fn advance(&mut self) {
        advance(&mut self.tracking, &self.lines);
    }
    pub fn save_state(&mut self) {
        self.saved = Save {
            tracking: self.tracking.clone(),
            time_stamps: self.time_stamps.clone(),
        };
    }
    pub fn reload_state(&mut self) {
        self.tracking = self.saved.tracking.clone();
        self.time_stamps = self.saved.time_stamps.clone();
    }
    pub fn load_timings(&mut self, path: String) {
        let file = std::fs::read_to_string(&path).unwrap();
        self.time_stamps.clear();
        for token in file.split(' ') {
            if token.is_empty() {
                break;
            }
            let time: f64 = token.parse().unwrap();
            self.time_stamps.push(time);
        }
        self.timings_path = Some(path);
    }
    pub fn save_timings(&self) {
        let path = self.timings_path.as_deref().unwrap_or("sub-timing.txt");
        let mut out = String::new();
        for time in &self.time_stamps {
            write!(&mut out, "{time} ").unwrap();
        }
        std::fs::write(path, out.as_bytes()).unwrap();
    }
    pub fn rewind(&mut self) {
        self.tracking = TrackingState::default();
    }
    pub fn clear(&mut self) {
        self.time_stamps.clear();
        self.rewind();
    }
    pub fn timings_reload_sentry(&mut self) -> Option<TimingsReloadSentry> {
        self.timings_path
            .clone()
            .map(|path| TimingsReloadSentry { subs: self, path })
    }
}

pub struct TimingsReloadSentry<'a> {
    subs: &'a mut SubsState,
    path: String,
}

impl TimingsReloadSentry<'_> {
    pub fn reload(mut self) {
        self.subs.load_timings(self.path.take());
    }
}

#[derive(Default, Clone)]
pub struct TrackingState {
    line_idx: usize,
    seg_idx: usize,
    /// Contains the currently timed texts for the tracks
    pub accumulators: Vec<String>,
    /// Contains the texts for the tracks for the current line in the song
    pub static_line_tracks: Vec<String>,
    /// Which characters have furigana
    pub static_furigana_indices: FuriMap,
    /// Which characters have furigana
    pub timed_furigana_indices: FuriMap,
    clear_next: bool,
    pub timestamp_tracker: usize,
    /// Used to have an empty period between two lines
    wait_next_line: bool,
    static_furi_debt: FuriDebt,
    timed_furi_debt: FuriDebt,
}

pub type FuriMap = HashMap<usize, HashMap<usize, Vec<String>>>;

fn advance(tracking: &mut TrackingState, lines: &[kashimark::Line]) {
    if tracking.wait_next_line {
        tracking.wait_next_line = false;
        tracking.accumulators.clear();
        tracking.static_line_tracks.clear();
        tracking.static_furigana_indices.clear();
        tracking.timed_furigana_indices.clear();
        return;
    }
    if tracking.clear_next {
        tracking.accumulators.clear();
        tracking.clear_next = false;
    }
    if let Some(line) = lines.get(tracking.line_idx) {
        if tracking.accumulators.len() < line.tracks.len() {
            tracking.accumulators = vec![String::new(); line.tracks.len()];
            tracking.static_line_tracks = vec![String::new(); line.tracks.len()];
        }
        for (i, ((track, accum), static_line)) in line
            .tracks
            .iter()
            .zip(tracking.accumulators.iter_mut())
            .zip(tracking.static_line_tracks.iter_mut())
            .enumerate()
        {
            match track {
                kashimark::Track::Timing(timing_track) => {
                    static_line.clear();
                    for seg in &timing_track.segments {
                        write_seg(
                            static_line,
                            seg,
                            i,
                            &mut tracking.static_furigana_indices,
                            &mut tracking.static_furi_debt,
                        );
                    }
                    let seg = &timing_track.segments[tracking.seg_idx];
                    write_seg(
                        accum,
                        seg,
                        i,
                        &mut tracking.timed_furigana_indices,
                        &mut tracking.timed_furi_debt,
                    );
                }
                kashimark::Track::Raw(a) => *accum = a.to_string(),
            }
        }
        tracking.seg_idx += 1;
        if tracking.seg_idx >= line.segment_count {
            tracking.clear_next = true;
            tracking.seg_idx = 0;
            tracking.line_idx += 1;
            tracking.wait_next_line = true;
        }
    }
}

#[derive(Default, Clone)]
struct FuriDebt {
    track_idx: usize,
    char_idx: usize,
    debt: VecDeque<String>,
}

fn write_seg(
    dest: &mut String,
    seg: &kashimark::TimedSegOrFill,
    track_idx: usize,
    furi: &mut FuriMap,
    furi_debt: &mut FuriDebt,
) {
    match seg {
        kashimark::TimedSegOrFill::Seg(timed_segment) => {
            write!(dest, "{}", timed_segment.text).unwrap();
            if !timed_segment.furigana.is_empty() {
                let idx_furi_map = furi.entry(track_idx).or_default();
                let last_idx = dest.chars().count().saturating_sub(1);
                let furi_vec = idx_furi_map.entry(last_idx).or_default();
                let (first, rest) = timed_segment.furigana.split_first().unwrap();
                *furi_vec = vec![first.clone()];
                furi_debt.track_idx = track_idx;
                furi_debt.char_idx = last_idx;
                furi_debt.debt = rest.to_vec().into();
            }
        }
        kashimark::TimedSegOrFill::Fill => {
            if let Some(furi_part) = furi_debt.debt.pop_front() {
                let idx_furi_map = furi.get_mut(&furi_debt.track_idx).unwrap();
                let furi_vec = idx_furi_map.get_mut(&furi_debt.char_idx).unwrap();
                furi_vec.push(furi_part);
            }
        }
    }
}
