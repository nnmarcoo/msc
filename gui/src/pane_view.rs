use iced::Element;
use msc_core::{Player, Track};
use std::cell::RefCell;
use std::fmt;

use crate::app::Message;

pub trait PaneView: fmt::Debug {
    fn update(&mut self, player: &Player);

    fn view<'a>(
        &'a self,
        player: &'a Player,
        volume: f32,
        hovered_track: &Option<i64>,
        seeking_position: Option<f32>,
        cached_tracks: &'a RefCell<Option<Vec<Track>>>,
        cached_albums: &'a RefCell<
            Option<Vec<(i64, String, Option<String>, Option<u32>, Option<String>)>>,
        >,
    ) -> Element<'a, Message>;

    fn title(&self) -> &str;

    fn clone_box(&self) -> Box<dyn PaneView>;
}

impl Clone for Box<dyn PaneView> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
