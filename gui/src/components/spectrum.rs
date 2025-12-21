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
        Canvas::new(Spectrum {
            bins: viz_data.frequency_bins,
        })
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(10)
    .into()
}

struct Spectrum {
    bins: Vec<f32>,
}

impl canvas::Program<Message> for Spectrum {
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
        let bar_color = palette.primary.strong.color;
        let background_color = palette.background.base.color;

        frame.fill_rectangle(iced::Point::ORIGIN, bounds.size(), background_color);

        if self.bins.is_empty() {
            return vec![frame.into_geometry()];
        }

        let bar_width = bounds.width / self.bins.len() as f32;
        let max_height = bounds.height;

        for (i, &amplitude) in self.bins.iter().enumerate() {
            let height = amplitude * max_height;
            let x = i as f32 * bar_width + 2.0;
            let y = bounds.height - height;

            let intensity = amplitude.clamp(0.0, 1.0);
            let color = Color::from_rgb(
                bar_color.r * intensity + (1.0 - intensity) * 0.2,
                bar_color.g * intensity + (1.0 - intensity) * 0.2,
                bar_color.b * intensity + (1.0 - intensity) * 0.2,
            );

            frame.fill_rectangle(
                iced::Point::new(x, y),
                Size::new(bar_width - 4.0, height),
                color,
            );
        }

        vec![frame.into_geometry()]
    }
}
