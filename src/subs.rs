use {
    crate::time_fmt::AssTimeFmt,
    egui_sf2g::egui::{TextBuffer, ahash::HashMap},
    std::{collections::VecDeque, fmt::Write as _, path::PathBuf},
};

pub struct SubsState {
    lines: Vec<kashimark::Line>,
    pub tracking: TrackingState,
    pub saved: Save,
    timings_path: Option<String>,
    pub time_stamps: Vec<f64>,
    path: PathBuf,
}

#[derive(Default)]
pub struct Save {
    tracking: TrackingState,
    time_stamps: Vec<f64>,
    pub mpv_position: f64,
}

const FW_SP: char = char::from_u32(0x3000).unwrap();

fn write_at_char_idx(s: String, idx: usize, ch: char) -> String {
    let mut old_iter = s.chars();
    let mut new = String::new();
    for _ in 0..idx {
        new.push(old_iter.next().unwrap());
    }
    old_iter.next();
    new.push(ch);
    new.extend(old_iter);
    new
}

#[test]
fn test_write_at_char_idx() {
    assert_eq!(write_at_char_idx("hello".to_string(), 3, 'a'), "helao");
}

impl SubsState {
    pub fn new(lines: Vec<kashimark::Line>, path: PathBuf) -> Self {
        Self {
            lines,
            tracking: TrackingState::default(),
            saved: Save::default(),
            time_stamps: Vec::new(),
            timings_path: None,
            path,
        }
    }
    pub fn advance(&mut self) {
        advance(&mut self.tracking, &self.lines);
    }
    pub fn save_state(&mut self, mpv_pos: f64) {
        self.saved = Save {
            tracking: self.tracking.clone(),
            time_stamps: self.time_stamps.clone(),
            mpv_position: mpv_pos,
        };
    }
    pub fn reload_state(&mut self) {
        self.tracking = self.saved.tracking.clone();
        // We don't want to overwrite our potentially existing timestamps with an empty one
        if !self.saved.time_stamps.is_empty() {
            self.time_stamps = self.saved.time_stamps.clone();
        }
    }
    pub fn load_timings(&mut self, path: String) -> anyhow::Result<()> {
        let file = std::fs::read_to_string(&path)?;
        self.time_stamps.clear();
        for token in file.split(' ') {
            if token.is_empty() {
                break;
            }
            let time: f64 = token.parse()?;
            self.time_stamps.push(time);
        }
        self.timings_path = Some(path);
        Ok(())
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
    pub fn timings_reload_sentry(&'_ mut self) -> Option<TimingsReloadSentry<'_>> {
        self.timings_path
            .clone()
            .map(|path| TimingsReloadSentry { subs: self, path })
    }
    pub fn write_ass(&mut self, path: &str, video_w: i64, video_h: i64) {
        let ass = self.gen_ass(video_w, video_h);
        std::fs::write(path, ass.as_bytes()).unwrap()
    }
    fn gen_ass(&mut self, video_w: i64, video_h: i64) -> String {
        let mut ass = String::new();
        ass.push_str(&format!("\
            [Script Info]\n\
            ScriptType: v4.00+\n\
            ScaledBorderAndShadow: yes\n\
            YCbCr Matrix: None\n\
            PlayResX: {video_w}\n\
            PlayResY: {video_h}\n\
            LayoutResX: {video_w}\n\
            LayoutResY: {video_h}\n\
            \n\
            [V4+ Styles]\n\
            Format: Name, Fontname, Fontsize,\
                     PrimaryColour, SecondaryColour, OutlineColour, BackColour,\
                     Bold, Italic, Underline, StrikeOut,\
                     ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n",
        ));
        let mut track_ids = Vec::new();
        for line in &self.lines {
            for track in &line.tracks {
                track_ids.push(track.id);
            }
        }
        track_ids.sort();
        track_ids.dedup();
        for track_id in track_ids {
            ass.push_str(&format!(
                "Style: Static{track_id},DejaVu Sans,44,\
                                   &H00AAAAAA,&H00000000,&H00000000,&H00000000,\
                                   0,0,0,0,\
                                   100.0,100.0,0.0,0.0,1,1.0,1.0,2,0,0,0,0\n"
            ));
            ass.push_str(&format!(
                "Style: Accum{track_id},DejaVu Sans,44,\
                                   &H00FFFFFF, &H00000000, &H00000000, &H00000000,\
                                   0,0,0,0,\
                                   100.0,100.0,0.0,0.0,1,1.0,1.0,2,0,0,0,1\n"
            ));
        }
        ass.push_str(
            "Style: StaticFuri,DejaVu Sans,22.2,\
                                   &H00AAAAAA, &H00000000, &H00000000, &H00000000,\
                                   0,0,0,0,\
                                   100.0,100.0,0.0,0.0,1,1.0,1.0,2,0,0,0,1\n",
        );
        ass.push_str(
            "Style: AccumFuri,DejaVu Sans,22.2,\
                                   &H00FFFFFF, &H00000000, &H00000000, &H00000000,\
                                   0,0,0,0,\
                                   100.0,100.0,0.0,0.0,1,1.0,1.0,2,0,0,0,1\n",
        );
        ass.push_str(concat!(
            "\n",
            "[Events]\n",
            "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        ));
        let time_stamps = self.time_stamps.clone();
        for [st, et] in time_stamps.array_windows() {
            self.advance();
            for (tid, track) in &self.tracking.static_line_tracks {
                let mut furi_line: String =
                    std::iter::repeat_n(FW_SP, track.chars().count() * 2).collect();
                if let Some(furis) = self.tracking.static_furigana_indices.get(tid) {
                    for (idx, furis) in furis {
                        let mut i = 0;
                        for furi in furis {
                            for ch in furi.chars() {
                                furi_line = write_at_char_idx(furi_line, (*idx * 2) + i, ch);
                                i += 1;
                            }
                        }
                    }
                }
                writeln!(
                    &mut ass,
                    "Dialogue: 0,{start},{end},Static{tid},,0,0,0,,{track}",
                    start = AssTimeFmt(*st),
                    end = AssTimeFmt(*et),
                )
                .unwrap();
                writeln!(
                    &mut ass,
                    "Dialogue: 0,{start},{end},StaticFuri,,0,0,0,,{furi_line}",
                    start = AssTimeFmt(*st),
                    end = AssTimeFmt(*et),
                )
                .unwrap();
            }
            for ((tid, track), (_sid, static_)) in self
                .tracking
                .accumulators
                .iter()
                .zip(&self.tracking.static_line_tracks)
            {
                let mut furi_line: String =
                    std::iter::repeat_n(FW_SP, static_.chars().count() * 2).collect();
                if let Some(timed_furis) = self.tracking.timed_furigana_indices.get(tid) {
                    for (idx, furis) in timed_furis {
                        let mut i = 0;
                        for furi in furis {
                            for ch in furi.chars() {
                                furi_line = write_at_char_idx(furi_line, (*idx * 2) + i, ch);
                                i += 1;
                            }
                        }
                    }
                }
                if static_.is_empty() {
                    writeln!(
                        &mut ass,
                        "Dialogue: 1,{start},{end},Accum{tid},,0,0,0,,{track}",
                        start = AssTimeFmt(*st),
                        end = AssTimeFmt(*et),
                    )
                    .unwrap();
                } else {
                    let mut transparented = static_[..track.len()].to_string();
                    transparented.push_str("{\\alpha&HFF&}");
                    transparented.push_str(&static_[track.len()..]);
                    writeln!(
                        &mut ass,
                        "Dialogue: 1,{start},{end},Accum{tid},,0,0,0,,{transparented}",
                        start = AssTimeFmt(*st),
                        end = AssTimeFmt(*et),
                    )
                    .unwrap();
                }
                writeln!(
                    &mut ass,
                    "Dialogue: 1,{start},{end},AccumFuri,,0,0,0,,{furi_line}",
                    start = AssTimeFmt(*st),
                    end = AssTimeFmt(*et),
                )
                .unwrap();
            }
        }
        ass
    }

    pub(crate) fn reload(&mut self) -> anyhow::Result<()> {
        self.lines = kashimark::parse(&std::fs::read_to_string(&self.path)?)?;
        // Not sure what else to do to correctly reset things
        //self.rewind();
        Ok(())
    }
}

pub struct TimingsReloadSentry<'a> {
    subs: &'a mut SubsState,
    path: String,
}

impl TimingsReloadSentry<'_> {
    pub fn reload(mut self) -> anyhow::Result<()> {
        self.subs.load_timings(self.path.take())
    }
}

#[derive(Default, Clone)]
pub struct TrackingState {
    line_idx: usize,
    seg_idx: usize,
    /// Contains the currently timed texts for the tracks
    pub accumulators: Vec<(u8, String)>,
    /// Contains the texts for the tracks for the current line in the song
    pub static_line_tracks: Vec<(u8, String)>,
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

pub type FuriMap = HashMap<u8, HashMap<usize, Vec<String>>>;

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
            tracking.accumulators = Vec::new();
            tracking.static_line_tracks = Vec::new();
            for track in &line.tracks {
                tracking.accumulators.push((track.id, String::new()));
                tracking.static_line_tracks.push((track.id, String::new()));
            }
        }
        for ((track, (aid, accum)), (sid, static_line)) in line
            .tracks
            .iter()
            .zip(tracking.accumulators.iter_mut())
            .zip(tracking.static_line_tracks.iter_mut())
        {
            match &track.data {
                kashimark::TrackData::Timing(timing_track) => {
                    static_line.clear();
                    for seg in &timing_track.segments {
                        write_seg(
                            static_line,
                            seg,
                            *sid,
                            &mut tracking.static_furigana_indices,
                            &mut tracking.static_furi_debt,
                        );
                    }
                    let Some(seg) = &timing_track.segments.get(tracking.seg_idx) else {
                        eprintln!("Can't advance subs... Probably at end");
                        return;
                    };
                    write_seg(
                        accum,
                        seg,
                        *aid,
                        &mut tracking.timed_furigana_indices,
                        &mut tracking.timed_furi_debt,
                    );
                }
                kashimark::TrackData::Raw(a) => *accum = a.to_string(),
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
    track_id: u8,
    char_idx: usize,
    debt: VecDeque<String>,
}

fn write_seg(
    dest: &mut String,
    seg: &kashimark::TimedSegOrFill,
    track_id: u8,
    furi: &mut FuriMap,
    furi_debt: &mut FuriDebt,
) {
    match seg {
        kashimark::TimedSegOrFill::Seg(timed_segment) => {
            write!(dest, "{}", timed_segment.text).unwrap();
            if !timed_segment.furigana.is_empty() {
                let idx_furi_map = furi.entry(track_id).or_default();
                let last_idx = dest.chars().count().saturating_sub(1);
                let furi_vec = idx_furi_map.entry(last_idx).or_default();
                let (first, rest) = timed_segment.furigana.split_first().unwrap();
                *furi_vec = vec![first.clone()];
                furi_debt.track_id = track_id;
                furi_debt.char_idx = last_idx;
                furi_debt.debt = rest.to_vec().into();
            }
        }
        kashimark::TimedSegOrFill::Fill => {
            if let Some(furi_part) = furi_debt.debt.pop_front() {
                let idx_furi_map = furi.get_mut(&furi_debt.track_id).unwrap();
                let furi_vec = idx_furi_map.get_mut(&furi_debt.char_idx).unwrap();
                furi_vec.push(furi_part);
            }
        }
    }
}
