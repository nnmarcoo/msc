use iced::Element;
use msc_core::{Album, Player, Track};
use std::cell::RefCell;
use std::fmt;

use crate::app::Message;
use crate::art_cache::ArtCache;

pub trait PaneView: fmt::Debug {
    fn update(&mut self, player: &Player, art: &mut ArtCache);

    fn view<'a>(
        &'a self,
        player: &'a Player,
        volume: f32,
        hovered_track: &Option<i64>,
        seeking_position: Option<f32>,
        cached_tracks: &'a RefCell<Option<Vec<Track>>>,
        cached_albums: &'a RefCell<Option<Vec<Album>>>,
        art: &'a ArtCache,
    ) -> Element<'a, Message>;

    fn title(&self) -> &str;

    /// Called when the library is rescanned so panes can drop stale state.
    fn invalidate_cache(&mut self) {}

    fn clone_box(&self) -> Box<dyn PaneView>;
}

impl Clone for Box<dyn PaneView> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
