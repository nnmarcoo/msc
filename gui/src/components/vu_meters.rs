use iced::{
    Color, Element, Length, Rectangle, Size, Theme, mouse,
    widget::{
        canvas::{self, Canvas, Frame, Geometry},
        container,
    },
};
use msc_core::Player;

use crate::app::Message;

pub fn view<'a>(player: &Player) -> Element<'a, Message> {
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
    .padding(10)
    .into()
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
        let bar_color = palette.primary.strong.color;
        let meter_background_color = palette.background.weak.color;

        frame.fill_rectangle(iced::Point::ORIGIN, bounds.size(), background_color);

        let spacing = 10.0;
        let meter_width = (bounds.width - spacing) / 2.0;
        let meter_height = bounds.height;

        let is_wide = bounds.width > bounds.height;

        if is_wide {
            draw_horizontal_meter(
                &mut frame,
                0.0,
                0.0,
                meter_width,
                meter_height,
                self.left,
                bar_color,
                meter_background_color,
            );

            draw_horizontal_meter(
                &mut frame,
                meter_width + spacing,
                0.0,
                meter_width,
                meter_height,
                self.right,
                bar_color,
                meter_background_color,
            );
        } else {
            draw_vertical_meter(
                &mut frame,
                0.0,
                0.0,
                meter_width,
                meter_height,
                self.left,
                bar_color,
                meter_background_color,
            );

            draw_vertical_meter(
                &mut frame,
                meter_width + spacing,
                0.0,
                meter_width,
                meter_height,
                self.right,
                bar_color,
                meter_background_color,
            );
        }

        vec![frame.into_geometry()]
    }
}

fn draw_horizontal_meter(
    frame: &mut Frame,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    level: f32,
    bar_color: Color,
    meter_background: Color,
) {
    frame.fill_rectangle(
        iced::Point::new(x, y),
        Size::new(width, height),
        meter_background,
    );

    let level_width = (level.clamp(0.0, 1.0) * width).max(0.0);

    let intensity = level.clamp(0.0, 1.0);
    let color = Color::from_rgb(
        bar_color.r * intensity + (1.0 - intensity) * 0.2,
        bar_color.g * intensity + (1.0 - intensity) * 0.2,
        bar_color.b * intensity + (1.0 - intensity) * 0.2,
    );

    frame.fill_rectangle(
        iced::Point::new(x, y),
        Size::new(level_width, height),
        color,
    );
}

fn draw_vertical_meter(
    frame: &mut Frame,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    level: f32,
    bar_color: Color,
    meter_background: Color,
) {
    frame.fill_rectangle(
        iced::Point::new(x, y),
        Size::new(width, height),
        meter_background,
    );

    let level_height = (level.clamp(0.0, 1.0) * height).max(0.0);
    let level_y = y + height - level_height;

    let intensity = level.clamp(0.0, 1.0);
    let color = Color::from_rgb(
        bar_color.r * intensity + (1.0 - intensity) * 0.2,
        bar_color.g * intensity + (1.0 - intensity) * 0.2,
        bar_color.b * intensity + (1.0 - intensity) * 0.2,
    );

    frame.fill_rectangle(
        iced::Point::new(x, level_y),
        Size::new(width, level_height),
        color,
    );
}
