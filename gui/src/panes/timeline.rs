use iced::widget::{column, text};
use iced::{Element, Theme};
use msc_core::{Player, Track};
use std::cell::RefCell;

use crate::app::Message;
use crate::pane_view::PaneView;

#[derive(Debug, Clone)]
pub struct TimelinePane;

impl TimelinePane {
    pub fn new() -> Self {
        Self
    }
}

impl PaneView for TimelinePane {
    fn update(&mut self, _player: &Player) {
        // No state to update
    }

    fn view<'a>(
        &'a self,
        _player: &'a Player,
        _volume: f32,
        _hovered_track: &Option<i64>,
        _seeking_position: Option<f32>,
        _cached_tracks: &'a RefCell<Option<Vec<Track>>>,
        _cached_albums: &'a RefCell<
            Option<Vec<(i64, String, Option<String>, Option<u32>, Option<String>)>>,
        >,
    ) -> Element<'a, Message> {
        column![
            text("Timeline / Seek Bar")
                .size(16)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.extended_palette().background.base.text),
                }),
            text("0:00 ━━━━━━━━━━ 3:45")
                .size(14)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.extended_palette().background.base.text),
                }),
        ]
        .spacing(10)
        .padding(20)
        .into()
    }

    fn title(&self) -> &str {
        "Timeline"
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}
