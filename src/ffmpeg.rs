use std::{
    fmt::Write,
    process::{Child, Command, Stdio},
};

use egui_sfml::egui::TextBuffer;
use thiserror::Error;

use crate::{source, SourceMarkers};

pub(crate) fn invoke(
    input: &str,
    markers: &SourceMarkers,
    src_info: &source::Info,
) -> anyhow::Result<Child> {
    let resolved = resolve(input, markers, src_info)?;
    Ok(Command::new("ffmpeg")
        .args(resolved)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?)
}

#[derive(Error, Debug)]
enum ResolveError {
    #[error("{0}")]
    Parse(#[from] ParseError),
    #[error("{0}")]
    ShellParseError(#[from] shell_words::ParseError),
    #[error("Mising item: {name}")]
    MissingItem { name: String },
    #[error("Format error: {0}")]
    FmtError(#[from] std::fmt::Error),
}

fn resolve(
    input: &str,
    markers: &SourceMarkers,
    src_info: &source::Info,
) -> Result<Vec<String>, ResolveError> {
    let words = shell_words::split(input)?;
    let mut out = Vec::new();
    for word in words {
        let tokens = tokenize_word(&word)?;
        out.extend_from_slice(&resolve_word_tokens(&tokens, markers, src_info)?);
    }
    Ok(out)
}

/// Takes a token stream of word tokens, and turns it into one more more resolved strings
///
/// Example:
/// rect turns into: ["w:h:x:y"], single token
/// timespan turns into ["-ss", "begin", "-t", "duration"], 4 tokens
fn resolve_word_tokens(
    tokens: &[Token],
    markers: &SourceMarkers,
    src_info: &source::Info,
) -> Result<Vec<String>, ResolveError> {
    let mut resolved = Vec::new();
    let mut current_string = String::new();
    for tok in tokens {
        match tok {
            Token::Raw(raw) => current_string.push_str(raw),
            Token::SubsRect(name) => {
                let marker = markers
                    .rects
                    .iter()
                    .find(|marker| &marker.name == name)
                    .ok_or_else(|| ResolveError::MissingItem {
                        name: name.to_string(),
                    })?;
                write!(
                    &mut current_string,
                    "{}:{}:{}:{}",
                    marker.rect.dim.x, marker.rect.dim.y, marker.rect.pos.x, marker.rect.pos.y
                )?;
            }
            Token::SubsTimespan(name) => {
                let marker = markers
                    .timespans
                    .iter()
                    .find(|marker| &marker.name == name)
                    .ok_or_else(|| ResolveError::MissingItem {
                        name: name.to_string(),
                    })?;
                resolved.push("-ss".into());
                resolved.push(marker.timespan.begin.to_string());
                resolved.push("-t".into());
                resolved.push((marker.timespan.end - marker.timespan.begin).to_string());
            }
            Token::SubsInput => current_string.push_str(&src_info.path),
        }
    }
    if !current_string.is_empty() {
        resolved.push(current_string.take());
    }
    Ok(resolved)
}

enum Status {
    Init,
    SubsBegin,
    /// Period after r. or t.
    SubsCategAccess,
    /// The "meat" of the substitution
    SubsMeat,
}

enum SubsType {
    Rect,
    TimeSpan,
    Input,
}

struct ParseState {
    status: Status,
    subs_type: SubsType,
    token_begin: usize,
}

impl Default for ParseState {
    fn default() -> Self {
        Self {
            status: Status::Init,
            subs_type: SubsType::Rect,
            token_begin: 0,
        }
    }
}

#[derive(Error, Debug)]
enum ParseError {
    #[error("Unexpected token")]
    UnexpectedToken,
    #[error("Unexpected end")]
    UnexpectedEnd,
}

fn tokenize_word(word: &str) -> Result<Vec<Token>, ParseError> {
    let mut state = ParseState::default();
    let mut tokens = Vec::new();
    for (i, byte) in word.bytes().enumerate() {
        match state.status {
            Status::Init => {
                if byte == b'{' {
                    let raw = &word[state.token_begin..i];
                    if !raw.is_empty() {
                        tokens.push(Token::Raw(raw));
                    }
                    state.status = Status::SubsBegin;
                }
            }
            Status::SubsBegin => match byte {
                b'i' => {
                    state.status = Status::SubsMeat;
                    state.subs_type = SubsType::Input;
                    state.token_begin = i + 1;
                }
                b'r' => {
                    state.status = Status::SubsCategAccess;
                    state.subs_type = SubsType::Rect;
                }
                b't' => {
                    state.status = Status::SubsCategAccess;
                    state.subs_type = SubsType::TimeSpan;
                }
                _ => return Err(ParseError::UnexpectedToken),
            },
            Status::SubsCategAccess => {
                if byte == b'.' {
                    state.token_begin = i + 1;
                    state.status = Status::SubsMeat;
                }
            }
            Status::SubsMeat => {
                if byte == b'}' {
                    let raw = &word[state.token_begin..i];
                    let tok = match state.subs_type {
                        SubsType::Rect => Token::SubsRect(raw),
                        SubsType::TimeSpan => Token::SubsTimespan(raw),
                        SubsType::Input => Token::SubsInput,
                    };
                    tokens.push(tok);
                    state.token_begin = i + 1;
                    state.status = Status::Init;
                }
            }
        }
    }
    // Do end-of-input handling
    match state.status {
        Status::Init => {
            let substr = &word[state.token_begin..];
            if !substr.is_empty() {
                tokens.push(Token::Raw(substr));
            }
        }

        Status::SubsBegin | Status::SubsCategAccess | Status::SubsMeat => {
            return Err(ParseError::UnexpectedEnd)
        }
    }
    Ok(tokens)
}

#[derive(Debug, Clone)]
enum Token<'a> {
    Raw(&'a str),
    SubsRect(&'a str),
    SubsTimespan(&'a str),
    SubsInput,
}

#[test]
fn test_resolve() {
    use crate::coords::{VideoDim, VideoPos, VideoRect};
    use crate::{RectMarker, SourceMarkers, TimeSpan, TimespanMarker};
    let test_markers = SourceMarkers {
        rects: vec![RectMarker {
            rect: VideoRect {
                pos: VideoPos::new(0, 0),
                dim: VideoDim::new(100, 100),
            },
            name: "0".into(),
            color: [0., 0., 0.],
        }],
        timespans: vec![TimespanMarker {
            timespan: TimeSpan {
                begin: 10.0,
                end: 20.0,
            },
            name: "0".into(),
            color: [0., 0., 0.],
        }],
    };
    let test_src_info = source::Info {
        dim: VideoDim::new(0, 0),
        w_h_ratio: 0.0,
        duration: 0.0,
        time_pos: 0.0,
        path: "/home/my_video.mp4".into(),
    };
    assert_eq!(
        resolve("-i {i} {t.0} crop={r.0}", &test_markers, &test_src_info).unwrap(),
        vec![
            "-i".to_string(),
            "/home/my_video.mp4".to_string(),
            "-ss".to_string(),
            "10".to_string(),
            "-t".to_string(),
            "10".to_string(),
            "crop=100:100:0:0".to_string()
        ]
    );
}
