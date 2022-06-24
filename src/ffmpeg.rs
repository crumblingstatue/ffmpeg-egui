use std::fmt::Write;

use thiserror::Error;

use crate::{source, SourceMarkers};

pub(crate) fn invoke(input: &str, markers: &SourceMarkers, src_info: &source::Info) {
    let resolved = resolve(input, markers, src_info);
    eprintln!("{:?}", resolved);
}

#[derive(Error, Debug)]
enum ResolveError {
    #[error("{0}")]
    Parse(#[from] ParseError),
}

fn resolve(
    input: &str,
    markers: &SourceMarkers,
    src_info: &source::Info,
) -> Result<String, ResolveError> {
    let tokens = tokenize(input)?;
    let mut out = String::new();
    for tok in tokens {
        match tok {
            Token::Raw(raw) => out.push_str(raw),
            Token::SubsRect(name) => {
                let marker = markers
                    .rects
                    .iter()
                    .find(|marker| marker.name == name)
                    .unwrap();
                write!(
                    &mut out,
                    "{}:{}:{}:{}",
                    marker.rect.dim.x, marker.rect.dim.y, marker.rect.pos.x, marker.rect.pos.y
                )
                .unwrap();
            }
            Token::SubsTimespan(name) => {
                let marker = markers
                    .timespans
                    .iter()
                    .find(|marker| marker.name == name)
                    .unwrap();
                write!(
                    &mut out,
                    "-ss {} -t {}",
                    marker.timespan.begin,
                    marker.timespan.end - marker.timespan.begin
                )
                .unwrap();
            }
            Token::SubsInput => out.push_str(&src_info.path),
        }
    }
    Ok(out)
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
}

impl Default for ParseState {
    fn default() -> Self {
        Self {
            status: Status::Init,
            subs_type: SubsType::Rect,
        }
    }
}

#[derive(Error, Debug)]
enum ParseError {
    #[error("Unexpected token")]
    UnexpectedToken,
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut state = ParseState::default();
    let mut tok_begin = 0;
    for (i, byte) in input.bytes().enumerate() {
        match state.status {
            Status::Init => {
                if let b'{' = byte {
                    let slice = &input[tok_begin..i];
                    if !slice.is_empty() {
                        tokens.push(Token::Raw(slice));
                    }
                    state.status = Status::SubsBegin;
                }
            }
            Status::SubsBegin => match byte {
                b'r' => {
                    state.subs_type = SubsType::Rect;
                    state.status = Status::SubsCategAccess
                }
                b't' => {
                    state.subs_type = SubsType::TimeSpan;
                    state.status = Status::SubsCategAccess
                }
                b'i' => {
                    state.subs_type = SubsType::Input;
                    state.status = Status::SubsMeat;
                }
                _ => return Err(ParseError::UnexpectedToken),
            },
            Status::SubsCategAccess => {
                if byte != b'.' {
                    return Err(ParseError::UnexpectedToken);
                }
                tok_begin = i + 1;
                state.status = Status::SubsMeat;
            }
            Status::SubsMeat => {
                if byte == b'}' {
                    let slice = &input[tok_begin..i];
                    if !slice.is_empty() {
                        match state.subs_type {
                            SubsType::Rect => tokens.push(Token::SubsRect(slice)),
                            SubsType::TimeSpan => tokens.push(Token::SubsTimespan(slice)),
                            SubsType::Input => tokens.push(Token::SubsInput),
                        }
                    }
                    tok_begin = i + 1;
                    state.status = Status::Init;
                }
            }
        }
    }
    Ok(tokens)
}

#[derive(Debug)]
enum Token<'a> {
    Raw(&'a str),
    SubsRect(&'a str),
    SubsTimespan(&'a str),
    SubsInput,
}

#[test]
fn test_resolve() {
    use crate::coords::{VideoDim, VideoPos, VideoRect};
    use crate::{RectMarker, SourceMarkers, TimeSpan};
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
        &resolve("-i {i} {t.0} crop={r.0}", &test_markers, &test_src_info).unwrap(),
        "-i /home/my_video.mp4 -ss 10 -t 10 crop=100:100:0:0"
    );
}
