use iced::{
    mouse, Color, Element, Length, Rectangle, Size, Theme,
    widget::{
        canvas::{self, Canvas, Frame, Geometry},
        container,
    },
};
use msc_core::{Player, Track};
use std::cell::RefCell;

use crate::app::Message;
use crate::pane_view::PaneView;

#[derive(Debug, Clone)]
pub struct SpectrumPane;

impl SpectrumPane {
    pub fn new() -> Self {
        Self
    }
}

impl PaneView for SpectrumPane {
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
        let bins = viz_data.bins_smooth().to_vec();

        container(
            Canvas::new(Spectrum { bins })
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(10)
        .into()
    }

    fn title(&self) -> &str {
        "Spectrum"
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
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

        let spacing = (bar_width * 0.1).clamp(1.0, 4.0);

        for (i, &amplitude) in self.bins.iter().enumerate() {
            let height = amplitude * max_height;
            let x = i as f32 * bar_width + spacing / 2.0;
            let y = bounds.height - height;

            let intensity = amplitude.clamp(0.0, 1.0);
            let color = Color::from_rgb(
                bar_color.r * intensity + (1.0 - intensity) * 0.2,
                bar_color.g * intensity + (1.0 - intensity) * 0.2,
                bar_color.b * intensity + (1.0 - intensity) * 0.2,
            );

            frame.fill_rectangle(
                iced::Point::new(x, y),
                Size::new(bar_width - spacing, height),
                color,
            );
        }

        vec![frame.into_geometry()]
    }
}
