use iced::Element;
use iced::widget::space;
use msc_core::{Player, Track};
use std::cell::RefCell;

use crate::app::Message;
use crate::pane_view::PaneView;

#[derive(Debug, Clone)]
pub struct SettingsPane;

impl SettingsPane {
    pub fn new() -> Self {
        Self
    }
}

impl PaneView for SettingsPane {
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
        space().into()
    }

    fn title(&self) -> &str {
        "Settings"
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}
