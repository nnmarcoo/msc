use iced::widget::{column, text};
use iced::{Element, Theme};
use msc_core::Player;

use crate::app::Message;
use crate::art_cache::ArtCache;
use crate::pane_view::{PaneView, ViewContext};

#[derive(Debug, Clone)]
pub struct TimelinePane;

impl TimelinePane {
    pub fn new() -> Self {
        Self
    }
}

impl PaneView for TimelinePane {
    fn update(&mut self, _player: &Player, _art: &mut ArtCache) {}

    fn view<'a>(&'a self, _ctx: ViewContext<'a>) -> Element<'a, Message> {
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

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}
