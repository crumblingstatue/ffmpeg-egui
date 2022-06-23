use sfml::graphics::{
    Color, Font, RectangleShape, RenderTarget, RenderWindow, Shape, Text, Transformable,
};

use crate::{coords::Present, source, ui::EguiFriendlyColorExt, SourceMarkers, VideoDim};

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
    // Draw timeline
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
}
