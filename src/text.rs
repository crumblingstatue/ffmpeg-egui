use crate::{
    TimeSpan,
    coords::{Src, VideoPos},
};

#[derive(Clone)]
pub struct Text {
    pub string: String,
    pub pos: VideoPos<Src>,
    pub timespan: TimeSpan,
    pub size: u32,
    pub borderw: u16,
    pub font_path: String,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            string: "Sample text".into(),
            pos: VideoPos::new(0, 0),
            timespan: TimeSpan {
                begin: 0.,
                end: 100.,
            },
            size: 16,
            borderw: 0,
            font_path: String::default(),
        }
    }
}
