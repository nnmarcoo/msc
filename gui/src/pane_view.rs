use iced::Element;
use msc_core::{Album, Player, Track};
use std::cell::RefCell;
use std::fmt;

use crate::app::Message;
use crate::art_cache::ArtCache;

pub struct ViewContext<'a> {
    pub player: &'a Player,
    pub volume: f32,
    pub hovered_track: &'a Option<i64>,
    pub seeking_position: Option<f32>,
    pub cached_tracks: &'a RefCell<Option<Vec<Track>>>,
    pub cached_albums: &'a RefCell<Option<Vec<Album>>>,
    pub art: &'a ArtCache,
}

pub trait PaneView: fmt::Debug {
    fn update(&mut self, player: &Player, art: &mut ArtCache);

    fn view<'a>(&'a self, ctx: ViewContext<'a>) -> Element<'a, Message>;

    /// Called when the library is rescanned so panes can drop stale state.
    fn invalidate_cache(&mut self) {}

    fn clone_box(&self) -> Box<dyn PaneView>;
}

impl Clone for Box<dyn PaneView> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
