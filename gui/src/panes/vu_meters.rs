use iced::{
    Color, Element, Length, Rectangle, Size, Theme, mouse,
    widget::{
        canvas::{self, Canvas, Frame, Geometry, Path, Stroke},
        container,
    },
};
use msc_core::{Player, Track};
use std::cell::RefCell;

use crate::app::Message;
use crate::pane_view::PaneView;

#[derive(Debug, Clone)]
pub struct VUMetersPane;

impl VUMetersPane {
    pub fn new() -> Self {
        Self
    }
}

impl PaneView for VUMetersPane {
    fn update(&mut self, _player: &Player) {
        // No state to update
    }

    fn view<'a>(
        &'a self,
        player: &'a Player,
        _volume: f32,
        _hovered_track: &Option<i64>,
        _seeking_position: Option<f32>,
        _cached_tracks: &'a RefCell<Option<Vec<Track>>>,
        _cached_albums: &'a RefCell<
            Option<Vec<(i64, String, Option<String>, Option<u32>, Option<String>)>>,
        >,
    ) -> Element<'a, Message> {
        let viz_data = player.vis_data();

        container(
            Canvas::new(VUMeters {
                left: viz_data.peak_left,
                right: viz_data.peak_right,
            })
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .into()
    }

    fn title(&self) -> &str {
        "VU Meters"
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}

struct VUMeters {
    left: f32,
    right: f32,
}

impl canvas::Program<Message> for VUMeters {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let palette = theme.extended_palette();
        let background_color = palette.background.base.color;

        frame.fill_rectangle(iced::Point::ORIGIN, bounds.size(), background_color);

        let spacing = 20.0;
        let meter_width = (bounds.width - spacing) / 2.0;
        let meter_height = bounds.height;

        let is_wide = bounds.width > bounds.height;

        if is_wide {
            draw_horizontal_meter(
                &mut frame,
                theme,
                0.0,
                0.0,
                meter_width,
                meter_height,
                self.left,
            );

            draw_horizontal_meter(
                &mut frame,
                theme,
                meter_width + spacing,
                0.0,
                meter_width,
                meter_height,
                self.right,
            );
        } else {
            draw_vertical_meter(
                &mut frame,
                theme,
                0.0,
                0.0,
                meter_width,
                meter_height,
                self.left,
            );

            draw_vertical_meter(
                &mut frame,
                theme,
                meter_width + spacing,
                0.0,
                meter_width,
                meter_height,
                self.right,
            );
        }

        vec![frame.into_geometry()]
    }
}

fn draw_horizontal_meter(
    frame: &mut Frame,
    theme: &Theme,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    level: f32,
) {
    let palette = theme.extended_palette();

    let bar_height = height * 0.6;
    let bar_y = y + (height - bar_height) / 2.0;

    let bg_color = palette.background.weak.color;
    let border_color = palette.background.strong.color;

    frame.fill_rectangle(
        iced::Point::new(x, bar_y),
        Size::new(width, bar_height),
        bg_color,
    );

    let border_path = Path::rectangle(iced::Point::new(x, bar_y), Size::new(width, bar_height));
    frame.stroke(
        &border_path,
        Stroke::default().with_color(border_color).with_width(1.5),
    );

    let num_segments = 40;
    let segment_width = width / num_segments as f32;
    // Make spacing proportional to segment width (10% of segment width, min 1px, max 3px)
    let segment_spacing = (segment_width * 0.1).clamp(1.0, 3.0);
    let level_clamped = level.clamp(0.0, 1.0);

    for i in 0..num_segments {
        let seg_x = x + i as f32 * segment_width;
        let seg_width = segment_width - segment_spacing;

        let segment_level = i as f32 / num_segments as f32;

        if segment_level <= level_clamped {
            let color = get_meter_color(theme, segment_level);

            frame.fill_rectangle(
                iced::Point::new(seg_x + segment_spacing / 2.0, bar_y + 3.0),
                Size::new(seg_width, bar_height - 6.0),
                color,
            );
        }
    }

    let marker_color = palette.background.strong.color;
    let markers = vec![0.0, 0.25, 0.5, 0.75, 1.0];
    for &marker in &markers {
        let marker_x = x + marker * width;
        frame.fill_rectangle(
            iced::Point::new(marker_x, bar_y - 5.0),
            Size::new(1.0, 3.0),
            marker_color,
        );
    }
}

fn draw_vertical_meter(
    frame: &mut Frame,
    theme: &Theme,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    level: f32,
) {
    let palette = theme.extended_palette();

    let bar_width = width * 0.7;
    let bar_x = x + (width - bar_width) / 2.0;

    let bg_color = palette.background.weak.color;
    let border_color = palette.background.strong.color;

    frame.fill_rectangle(
        iced::Point::new(bar_x, y),
        Size::new(bar_width, height),
        bg_color,
    );

    let border_path = Path::rectangle(iced::Point::new(bar_x, y), Size::new(bar_width, height));
    frame.stroke(
        &border_path,
        Stroke::default().with_color(border_color).with_width(1.5),
    );

    let num_segments = 40;
    let segment_height = height / num_segments as f32;
    // Make spacing proportional to segment height (10% of segment height, min 1px, max 3px)
    let segment_spacing = (segment_height * 0.1).clamp(1.0, 3.0);
    let level_clamped = level.clamp(0.0, 1.0);

    for i in 0..num_segments {
        let seg_index = num_segments - 1 - i;
        let seg_y = y + seg_index as f32 * segment_height;
        let seg_height = segment_height - segment_spacing;

        let segment_level = i as f32 / num_segments as f32;

        if segment_level <= level_clamped {
            let color = get_meter_color(theme, segment_level);

            frame.fill_rectangle(
                iced::Point::new(bar_x + 3.0, seg_y + segment_spacing / 2.0),
                Size::new(bar_width - 6.0, seg_height),
                color,
            );
        }
    }

    let marker_color = palette.background.strong.color;
    let markers = vec![0.0, 0.25, 0.5, 0.75, 1.0];
    for &marker in &markers {
        let marker_y = y + height - (marker * height);
        frame.fill_rectangle(
            iced::Point::new(bar_x - 5.0, marker_y),
            Size::new(3.0, 1.0),
            marker_color,
        );
    }
}

fn get_meter_color(theme: &Theme, level: f32) -> Color {
    let palette = theme.extended_palette();
    let primary = palette.primary.strong.color;

    if level < 0.7 {
        let intensity = level / 0.7;
        Color::from_rgb(
            primary.r * intensity + (1.0 - intensity) * 0.2,
            primary.g * intensity + (1.0 - intensity) * 0.2,
            primary.b * intensity + (1.0 - intensity) * 0.2,
        )
    } else if level < 0.9 {
        let t = (level - 0.7) / 0.2;
        let danger = palette.danger.strong.color;
        Color::from_rgb(
            primary.r * (1.0 - t) + danger.r * t,
            primary.g * (1.0 - t) + danger.g * t,
            primary.b * (1.0 - t) + danger.b * t,
        )
    } else {
        palette.danger.strong.color
    }
}
