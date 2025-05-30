use {
    crate::{
        app::AppState,
        coords::{Dim, Present, VideoDim, VideoMag, VideoVector},
        mpv::{Mpv, properties::TimePos},
        sfml_integ::EguiFriendlyColorExt as _,
        source,
        time_fmt::FfmpegTimeFmt,
    },
    egui_sf2g::sf2g::{
        graphics::{
            Color, Font, Rect, RectangleShape, RenderTarget, RenderWindow, Shape, Text,
            Transformable,
        },
        system::Vector2,
        window::{Event, mouse},
    },
    sf2g::graphics::RenderStates,
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
            let time_pos = timeline_rect_timepos(timeline_rect, x, src_info);
            mpv.set_property::<TimePos>(time_pos);
        }
    }
}

fn timeline_rect_timepos(timeline_rect: Rect<i16>, x: i16, src_info: &source::Info) -> f64 {
    let x_offset = x - timeline_rect.left;
    let ratio: f64 = x_offset as f64 / timeline_rect.width as f64;
    ratio * src_info.duration
}

pub(crate) fn draw_overlay(
    rw: &mut RenderWindow,
    app_state: &AppState,
    pos_string: &str,
    font: &Font,
) {
    let mouse_pos = rw.mouse_position();
    let mut rs = RectangleShape::default();
    let video_present_dim = app_state
        .present
        .as_ref()
        .map_or(VideoVector::new(0, 0), |present| present.dim);
    // Rect markers
    for marker in &app_state.source_markers.rects {
        let dim = marker
            .rect
            .dim
            .to_present(app_state.src.dim, video_present_dim);
        rs.set_size((dim.x.into(), dim.y.into()));
        let pos = marker
            .rect
            .pos
            .to_present(app_state.src.dim, video_present_dim);
        rs.set_position((pos.x.into(), pos.y.into()));
        let mut fill_c = marker.color.to_sfml();
        fill_c.a = 180;
        rs.set_fill_color(fill_c);
        rw.draw_rectangle_shape(&rs, &RenderStates::DEFAULT);
    }
    // Timeline
    rs.set_outline_color(Color::WHITE);
    rs.set_outline_thickness(2.0);
    rs.set_fill_color(Color::TRANSPARENT);
    let timeline_rect = timeline_rect(app_state.video_area_max_dim);
    let timeline_rect_sf: Rect<f32> = timeline_rect.into_other();
    rs.set_position(timeline_rect_sf.position());
    rs.set_size(timeline_rect_sf.size());
    rw.draw_rectangle_shape(&rs, &RenderStates::DEFAULT);
    rs.set_fill_color(Color::WHITE);
    let completed_ratio = app_state.src.time_pos / app_state.src.duration;
    rs.set_size((
        timeline_rect_sf.width * completed_ratio as f32,
        TIMELINE_H.into(),
    ));
    rw.draw_rectangle_shape(&rs, &RenderStates::DEFAULT);
    // Timespan markers
    for marker in &app_state.source_markers.timespans {
        draw_timespan_marker(
            timeline_rect_sf.width,
            marker.timespan.begin / app_state.src.duration,
            &mut rs,
            app_state.video_area_max_dim,
            marker,
            rw,
        );
        draw_timespan_marker(
            timeline_rect_sf.width,
            marker.timespan.end / app_state.src.duration,
            &mut rs,
            app_state.video_area_max_dim,
            marker,
            rw,
        );
    }
    // Text overlay
    let mut text = Text::new(pos_string.to_owned(), font, 14);
    text.set_position((
        app_state.video_area_max_dim.x as f32 - 240.0,
        timeline_rect_sf.top - 20.0,
    ));
    text.draw(rw, &RenderStates::DEFAULT);
    if timeline_rect.contains(mouse_pos.as_other()) {
        let timepos = timeline_rect_timepos(timeline_rect, mouse_pos.x as i16, &app_state.src);
        text.set_position((timeline_rect_sf.left, timeline_rect_sf.top - 20.0));
        text.set_string(format!("Mouse time pos: {}", FfmpegTimeFmt(timepos)));
        text.draw(rw, &RenderStates::DEFAULT);
    }
    // Draw subs
    if let Some(subs) = &app_state.subs {
        text.set_character_size(20);
        text.set_position(0.);
        text.set_outline_color(Color::BLACK);
        text.set_outline_thickness(2.0);
        let gray = Color::rgb(138, 145, 150);
        text.set_fill_color(gray);
        for (tid, track) in subs.tracking.static_line_tracks.iter() {
            text.set_string(format!("{track} ({tid})"));
            text.draw(rw, &RenderStates::DEFAULT);
            if let Some(furis) = subs.tracking.static_furigana_indices.get(tid) {
                for (furi_idx, furis) in furis {
                    let pos = text.find_character_pos(*furi_idx);
                    let mut smol = Text::new(furis.join(""), font, 10);
                    smol.set_outline_color(Color::BLACK);
                    smol.set_fill_color(gray);
                    smol.set_outline_thickness(1.0);
                    smol.set_position(pos + Vector2::new(0.0, -11.));
                    smol.draw(rw, &RenderStates::DEFAULT);
                }
            }
            text.move_((0., 32.0));
        }
        text.set_position(0.);
        text.set_fill_color(Color::WHITE);
        for (tid, accum) in &subs.tracking.accumulators {
            text.set_string(accum.clone());
            text.draw(rw, &RenderStates::DEFAULT);
            if let Some(furis) = subs.tracking.timed_furigana_indices.get(tid) {
                for (furi_idx, furis) in furis {
                    let pos = text.find_character_pos(*furi_idx);
                    let mut smol = Text::new(furis.join(""), font, 10);
                    smol.set_outline_color(Color::BLACK);
                    smol.set_fill_color(Color::WHITE);
                    smol.set_outline_thickness(1.0);
                    smol.set_position(pos + Vector2::new(0.0, -11.));
                    smol.draw(rw, &RenderStates::DEFAULT);
                }
            }
            text.move_((0., 32.0));
        }
    }
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
    rw.draw_rectangle_shape(&*rs, &RenderStates::DEFAULT);
}
