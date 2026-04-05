use iced::Element;
use iced::widget::space;
use msc_core::Player;

use crate::app::Message;
use crate::art_cache::ArtCache;
use crate::pane_view::{PaneView, ViewContext};

#[derive(Debug, Clone)]
pub struct EmptyPane;

impl EmptyPane {
    pub fn new() -> Self {
        Self
    }
}

impl PaneView for EmptyPane {
    fn update(&mut self, _player: &Player, _art: &mut ArtCache) {}

    fn view<'a>(&'a self, _ctx: ViewContext<'a>) -> Element<'a, Message> {
        space().into()
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}
