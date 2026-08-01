//! The search pane: one field that filters every pane listing tracks.
//!
//! The query is app state rather than pane state, so this pane holds nothing of
//! its own and two search panes show the same text. That is the intended
//! behavior, not a limitation: the query describes which tracks are interesting
//! right now, which is a fact about the library and not about a pane. See
//! [`crate::browsing`].
//!
//! An untouched pane and a cleared one are the same thing. An empty query means
//! no filter, so the library shows everything either way and there is no state
//! distinguishing "never typed" from "typed then cleared".
//!
//! The count is of tracks the query currently matches, so the pane reports the
//! effect of the filter even when no list is visible beside it. It arrives as a
//! number rather than being counted here: the filtered rows are built once per
//! frame for every pane that needs them, so a search pane beside a library pane
//! runs the search once between them and not twice.

use iced::widget::container;
use iced::{Element, Length};

use crate::app::Message;
use crate::browsing::Context;
use crate::styles::PAD;
use crate::widgets::search_bar::SearchBar;

pub fn view(tracks: Context<'_>, matched: usize) -> Element<'_, Message> {
    let bar = SearchBar::new(
        tracks.search,
        Message::SearchChanged,
        Message::SearchChanged(String::new()),
    )
    .count(matched);

    container(bar)
        .padding(PAD)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
