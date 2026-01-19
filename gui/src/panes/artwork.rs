use iced::{Element, Length};
use msc_core::{Player, Track};
use std::cell::RefCell;

use crate::app::Message;
use crate::pane_view::PaneView;
use crate::widgets::ArtworkImage;

#[derive(Debug, Clone)]
pub struct ArtworkPane {
    artwork: ArtworkImage,
    requested_size: RefCell<u32>,
}

impl ArtworkPane {
    pub fn new() -> Self {
        Self {
            artwork: ArtworkImage::new(512),
            requested_size: RefCell::new(512),
        }
    }
}

impl PaneView for ArtworkPane {
    fn update(&mut self, player: &Player) {
        let requested = *self.requested_size.borrow();
        if requested > 0 && requested != self.artwork.requested_size {
            self.artwork = ArtworkImage::new(requested);
        }

        self.artwork
            .update(player, player.clone_current_track().as_ref());
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
        let requested_size = &self.requested_size;

        iced::widget::responsive(move |size| {
            *requested_size.borrow_mut() =
                (size.width.max(size.height).ceil() as u32).clamp(64, 2048);

            self.artwork.view_with_background()
        })
        .into()
    }

    fn title(&self) -> &str {
        "Artwork"
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}
