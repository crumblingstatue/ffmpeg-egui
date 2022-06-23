use sfml::graphics::{
    Color, Font, Rect, RectangleShape, RenderTarget, RenderWindow, Shape, Text, Transformable,
};

use crate::{coords::translate_up, source, VideoDim};

pub(crate) fn draw_overlay(
    rw: &mut RenderWindow,
    pos_string: &String,
    font: &sfml::SfBox<Font>,
    rects: &Vec<Rect<u16>>,
    src_info: &source::Info,
    video_present_dim: VideoDim,
) {
    rw.draw(&Text::new(pos_string, font, 32));
    let mut rs = RectangleShape::default();
    rs.set_fill_color(Color::rgba(250, 250, 200, 128));
    for rect in rects {
        let (w, h) = translate_up(
            rect.width as i32,
            rect.height as i32,
            src_info.dim,
            video_present_dim,
        );
        rs.set_size((w as f32, h as f32));
        let (x, y) = translate_up(
            rect.left as i32,
            rect.top as i32,
            src_info.dim,
            video_present_dim,
        );
        rs.set_position((x as f32, y as f32));
        rw.draw(&rs);
    }
}
