use sfml::graphics::{
    Color, Font, RectangleShape, RenderTarget, RenderWindow, Shape, Text, Transformable,
};

use crate::{
    coords::{Dim, Present, VideoVector},
    source,
    ui::EguiFriendlyColorExt,
    SourceMarkers, VideoDim,
};

pub(crate) fn draw_overlay(
    rw: &mut RenderWindow,
    pos_string: &String,
    font: &sfml::SfBox<Font>,
    source_markers: &SourceMarkers,
    src_info: &source::Info,
    video_present_dim: VideoDim<Present>,
    video_area_max_dim: VideoDim<Present>,
) {
    rw.draw(&Text::new(pos_string, font, 32));
    let mut rs = RectangleShape::default();
    // Rect markers
    for marker in &source_markers.rects {
        let dim = marker.rect.dim.to_present(src_info.dim, video_present_dim);
        rs.set_size((dim.x.into(), dim.y.into()));
        let pos = marker.rect.pos.to_present(src_info.dim, video_present_dim);
        rs.set_position((pos.x.into(), pos.y.into()));
        let mut fill_c = marker.color.to_sfml();
        *fill_c.alpha_mut() = 180;
        rs.set_fill_color(fill_c);
        rw.draw(&rs);
    }
    // Timeline
    rs.set_outline_color(Color::WHITE);
    rs.set_outline_thickness(2.0);
    rs.set_fill_color(Color::TRANSPARENT);
    rs.set_position((20.0, video_area_max_dim.y as f32 - 40.0));
    let full_w = video_area_max_dim.x as f32 - 40.0;
    rs.set_size((full_w, 20.0));
    rw.draw(&rs);
    rs.set_fill_color(Color::WHITE);
    let completed_ratio = src_info.time_pos / src_info.duration;
    rs.set_size((full_w * completed_ratio as f32, 20.0));
    rw.draw(&rs);
    // Timespan markers
    for marker in &source_markers.timespans {
        draw_timespan_marker(
            full_w,
            marker.timespan.begin / src_info.duration,
            &mut rs,
            video_area_max_dim,
            marker,
            rw,
        );
        draw_timespan_marker(
            full_w,
            marker.timespan.end / src_info.duration,
            &mut rs,
            video_area_max_dim,
            marker,
            rw,
        );
    }
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
