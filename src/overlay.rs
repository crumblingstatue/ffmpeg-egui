use sfml::graphics::{
    Color, Font, RectangleShape, RenderTarget, RenderWindow, Shape, Text, Transformable,
};

use crate::{coords::VideoRect, source, VideoDim};

pub(crate) fn draw_overlay(
    rw: &mut RenderWindow,
    pos_string: &String,
    font: &sfml::SfBox<Font>,
    rects: &Vec<VideoRect>,
    src_info: &source::Info,
    video_present_dim: VideoDim,
) {
    rw.draw(&Text::new(pos_string, font, 32));
    let mut rs = RectangleShape::default();
    rs.set_fill_color(Color::rgba(250, 250, 200, 128));
    for rect in rects {
        let dim = rect.dim.to_present(src_info.dim, video_present_dim);
        rs.set_size((dim.x.into(), dim.y.into()));
        let pos = rect.pos.to_present(src_info.dim, video_present_dim);
        rs.set_position((pos.x.into(), pos.y.into()));
        rw.draw(&rs);
    }
}
