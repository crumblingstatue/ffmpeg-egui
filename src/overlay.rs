use sfml::{
    graphics::{
        Color, Font, Rect, RectangleShape, RenderTarget, RenderWindow, Shape, Text, Transformable,
    },
    window::{mouse, Event},
};

use crate::{
    coords::{Dim, Present, VideoMag, VideoVector},
    mpv::{properties::TimePos, Mpv},
    source,
    ui::EguiFriendlyColorExt,
    SourceMarkers, VideoDim,
};

const TIMELINE_MARGIN: VideoMag = 20;
const TIMELINE_H: VideoMag = 12;

type VideoRect = Rect<VideoMag>;

pub fn handle_event(
    event: &Event,
    mpv: &Mpv,
    src_info: &source::Info,
    video_area_max_dim: VideoDim<Present>,
) {
    if let Event::MouseButtonPressed {
        button: mouse::Button::Left,
        x,
        y,
    } = *event
    {
        let x = x as VideoMag;
        let y = y as VideoMag;
        let timeline_rect = timeline_rect(video_area_max_dim);
        if timeline_rect.contains((x, y).into()) {
            let x_offset = x - timeline_rect.left;
            let ratio: f64 = x_offset as f64 / timeline_rect.width as f64;
            mpv.set_property::<TimePos>(ratio * src_info.duration);
        }
    }
}

pub(crate) fn draw_overlay(
    rw: &mut RenderWindow,
    pos_string: &String,
    font: &sfml::SfBox<Font>,
    source_markers: &SourceMarkers,
    src_info: &source::Info,
    video_present_dim: VideoDim<Present>,
    video_area_max_dim: VideoDim<Present>,
) {
    let mut rs = RectangleShape::default();
    // Rect markers
    for marker in &source_markers.rects {
        let dim = marker.rect.dim.to_present(src_info.dim, video_present_dim);
        rs.set_size((dim.x.into(), dim.y.into()));
        let pos = marker.rect.pos.to_present(src_info.dim, video_present_dim);
        rs.set_position((pos.x.into(), pos.y.into()));
        let mut fill_c = marker.color.to_sfml();
        fill_c.a = 180;
        rs.set_fill_color(fill_c);
        rw.draw(&rs);
    }
    // Timeline
    rs.set_outline_color(Color::WHITE);
    rs.set_outline_thickness(2.0);
    rs.set_fill_color(Color::TRANSPARENT);
    let timeline_rect: Rect<f32> = timeline_rect(video_area_max_dim).into_other();
    rs.set_position(timeline_rect.position());
    rs.set_size(timeline_rect.size());
    rw.draw(&rs);
    rs.set_fill_color(Color::WHITE);
    let completed_ratio = src_info.time_pos / src_info.duration;
    rs.set_size((
        timeline_rect.width * completed_ratio as f32,
        TIMELINE_H.into(),
    ));
    rw.draw(&rs);
    // Timespan markers
    for marker in &source_markers.timespans {
        draw_timespan_marker(
            timeline_rect.width,
            marker.timespan.begin / src_info.duration,
            &mut rs,
            video_area_max_dim,
            marker,
            rw,
        );
        draw_timespan_marker(
            timeline_rect.width,
            marker.timespan.end / src_info.duration,
            &mut rs,
            video_area_max_dim,
            marker,
            rw,
        );
    }
    // Test text overlay
    let mut mp_text = Text::new(pos_string, font, 14);
    mp_text.set_position((
        video_area_max_dim.x as f32 - 240.0,
        timeline_rect.top - 20.0,
    ));
    rw.draw(&mp_text);
}

fn timeline_rect(video_area_max_dim: VideoVector<Dim, Present>) -> VideoRect {
    let left = TIMELINE_MARGIN;
    let top = video_area_max_dim.y - TIMELINE_MARGIN;
    let width = video_area_max_dim.x - TIMELINE_MARGIN * 2;
    let height = TIMELINE_H;
    Rect::new(left, top, width, height)
}

fn draw_timespan_marker(
    full_w: f32,
    pos_ratio: f64,
    rs: &mut RectangleShape,
    video_area_max_dim: VideoVector<Dim, Present>,
    marker: &crate::TimespanMarker,
    rw: &mut RenderWindow,
) {
    let x = (full_w * pos_ratio as f32) + 20.0;
    rs.set_position((x, (video_area_max_dim.y - 60) as f32));
    rs.set_fill_color(marker.color.to_sfml());
    rs.set_size((3.0, 14.0));
    rs.set_outline_thickness(0.0);
    rw.draw(&*rs);
}
