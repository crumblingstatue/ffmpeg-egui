use sfml::graphics::{
    Color, Font, RectangleShape, RenderTarget, RenderWindow, Shape, Text, Transformable,
};

use crate::{
    coords::{VideoPos, VideoRect},
    source, VideoDim,
};

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
        let dim = VideoDim::present_from_src(
            VideoDim {
                width: rect.width,
                height: rect.height,
            },
            src_info.dim,
            video_present_dim,
        );
        rs.set_size((dim.width as f32, dim.height as f32));
        let pos = VideoPos::present_from_src(
            VideoPos {
                x: rect.left,
                y: rect.top,
            },
            src_info.dim,
            video_present_dim,
        );
        rs.set_position((pos.x as f32, pos.y as f32));
        rw.draw(&rs);
    }
}
