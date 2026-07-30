//! The artwork pane: the cover of the track being heard.
//!
//! `responsive` is what makes the size intelligent: the pane asks
//! [`crate::artwork::Cache`] for the size it is actually drawing at rather than
//! one fixed in advance, so a pane dragged larger sharpens instead of stretching.
//! It asks by its shorter side, the cover being squared into the space, and draws
//! whatever size comes back rather than assuming it got the one it named.
//!
//! [`State`] holds the handle last drawn and the pane falls back to it while the
//! cache has nothing, because a track change takes a frame or two to resolve and
//! showing the placeholder in that gap blinks between covers that are usually the
//! same image. Holding the previous one is right whenever the new cover matches
//! and is replaced within a frame or two when it does not, where the alternative
//! blinks every time, including all the times it need not have.
//!
//! That state belongs to the pane rather than to the cache because it is a fact
//! about what *this* pane drew last and not about any image: two panes at
//! different sizes hold different handles and must not share a slot. It needs no
//! clearing rule beyond the two answers that mean there is genuinely nothing to
//! draw, nothing playing and a track read to carry no art, since a pane's own
//! last frame cannot go stale for anyone else.
//!
//! The placeholder holds the same square the art will fill, so a cover arriving
//! changes only the contents of that square and never the pane's shape.

use std::cell::RefCell;

use iced::widget::{Space, center, container, image, responsive, svg};
use iced::{ContentFit, Element, Length};

use crate::app::Message;
use crate::artwork::Cache;
use crate::styles::{self, PAD};
use crate::tracks::Context;

const ICON_PANE: &[u8] = include_bytes!("../../../assets/icons/pane.svg");
const ICON_SCALE: f32 = 0.28;
const ICON_MAX: f32 = 64.0;

#[derive(Debug, Default)]
pub struct State {
    last: RefCell<Option<image::Handle>>,
}

impl State {
    fn bridge(&self, drawn: Option<image::Handle>, playing: bool) -> Option<image::Handle> {
        if let Some(handle) = drawn {
            *self.last.borrow_mut() = Some(handle.clone());
            return Some(handle);
        }
        if !playing {
            self.clear();
            return None;
        }
        self.last.borrow().clone()
    }

    fn clear(&self) {
        *self.last.borrow_mut() = None;
    }
}

pub fn view<'a>(
    tracks: Context<'a>,
    art: &'a Cache,
    state: Option<&'a State>,
) -> Element<'a, Message> {
    let playing = tracks
        .playing
        .and_then(|id| tracks.library.track(id).map(|track| (id, track.path())));

    responsive(move |size| {
        let edge = size.width.min(size.height) - PAD * 2.0;
        if edge <= 0.0 {
            return Space::new().into();
        }

        let drawn = playing.and_then(|(id, path)| art.request(id, path, edge));

        if let Some(state) = state
            && drawn.is_none()
            && playing.is_some_and(|(id, _)| art.resolved_empty(id))
        {
            state.clear();
        }

        let shown = match state {
            Some(state) => state.bridge(drawn, playing.is_some()),
            None => drawn,
        };

        let body: Element<'_, Message> = match shown {
            Some(handle) => image(handle)
                .content_fit(ContentFit::Contain)
                .filter_method(image::FilterMethod::Linear)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            None => placeholder(edge, playing.is_some()),
        };

        container(body)
            .padding(PAD)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    })
    .into()
}

fn placeholder<'a>(edge: f32, playing: bool) -> Element<'a, Message> {
    let mark: Element<'a, Message> = if playing {
        let icon = (edge * ICON_SCALE).min(ICON_MAX);
        svg(svg::Handle::from_memory(ICON_PANE))
            .style(styles::svg_style)
            .width(Length::Fixed(icon))
            .height(Length::Fixed(icon))
            .into()
    } else {
        Space::new().into()
    };

    container(center(mark))
        .width(Length::Fixed(edge))
        .height(Length::Fixed(edge))
        .style(styles::artwork_placeholder_style)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle(seed: u8) -> image::Handle {
        image::Handle::from_rgba(1, 1, vec![seed, 0, 0, 255])
    }

    #[test]
    fn a_drawn_cover_is_what_is_shown() {
        let state = State::default();
        let cover = handle(1);

        assert_eq!(
            state.bridge(Some(cover.clone()), true),
            Some(cover),
            "the pane drew something other than the art it was given"
        );
    }

    #[test]
    fn a_gap_while_the_next_track_resolves_keeps_the_last_cover() {
        let state = State::default();
        let cover = handle(1);
        let _ = state.bridge(Some(cover.clone()), true);

        assert_eq!(
            state.bridge(None, true),
            Some(cover),
            "a frame with nothing ready blinked to the placeholder"
        );
    }

    #[test]
    fn a_new_cover_replaces_the_held_one() {
        let state = State::default();
        let (first, second) = (handle(1), handle(2));

        let _ = state.bridge(Some(first), true);
        let _ = state.bridge(Some(second.clone()), true);

        assert_eq!(
            state.bridge(None, true),
            Some(second),
            "it held a stale cover"
        );
    }

    #[test]
    fn stopping_playback_drops_the_held_cover() {
        let state = State::default();
        let _ = state.bridge(Some(handle(1)), true);

        assert_eq!(state.bridge(None, false), None);
        assert_eq!(
            state.bridge(None, true),
            None,
            "the cover from before a stop came back for the next track"
        );
    }

    #[test]
    fn a_track_known_to_have_no_art_shows_the_placeholder() {
        let state = State::default();
        let _ = state.bridge(Some(handle(1)), true);

        state.clear();

        assert_eq!(
            state.bridge(None, true),
            None,
            "an art-less track kept showing the cover before it"
        );
    }

    #[test]
    fn two_panes_hold_their_own_covers() {
        let (wide, narrow) = (State::default(), State::default());
        let (big, small) = (handle(1), handle(2));

        let _ = wide.bridge(Some(big.clone()), true);
        let _ = narrow.bridge(Some(small.clone()), true);

        assert_eq!(wide.bridge(None, true), Some(big));
        assert_eq!(narrow.bridge(None, true), Some(small));
    }
}
