//! The library pane: every track, filtered by the shared query.
//!
//! The pane owns no search state of its own. Query, selection, and hover are
//! properties of a track rather than of a pane, so they live on the app and
//! arrive through [`Context`]; see [`crate::browsing`]. Two library panes
//! therefore show the same filter and the same highlights, which is what makes
//! hovering a row here light the same track up in the queue.
//!
//! The header sits outside the `scrollable` so it stays put while the rows move
//! beneath it. `menu_entries` names how many tracks each action will act on, so
//! a selection scrolled out of view is still legible in what the menu says it
//! will do; the count is dropped for one track, where it reads as noise.

use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Length, Theme};

use verse_core::Track;

use crate::app::Message;
use crate::browsing::Context;
use crate::styles::PAD;
use crate::widgets::context_menu::{ContextMenu, Entry};
use crate::widgets::track_list::{Op, TrackList, header};

pub fn view<'a>(tracks: Context<'a>, visible: &[i64]) -> Element<'a, Message> {
    if tracks.library.is_empty() {
        return empty_library();
    }

    let rows: Vec<&'a Track> = visible
        .iter()
        .filter_map(|&id| tracks.library.track(id))
        .collect();

    if rows.is_empty() {
        return column![header(), no_results(tracks.search)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    let selected = rows
        .iter()
        .filter_map(|track| track.id())
        .filter(|&id| tracks.selection.contains(id))
        .count();

    let list = TrackList::new(rows, tracks, |op| match op {
        Op::Clicked(index, click) => Message::RowClicked(index, click),
        Op::Activated(index) => Message::RowActivated(index),
        Op::RightClicked(index) => Message::RowRightClicked(index),
        Op::Hovered(id) => Message::TrackHovered(id),
        Op::SelectAll => Message::SelectAll,
    });

    let list = ContextMenu::new(list, menu_entries(selected));

    column![
        header(),
        scrollable(list)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn menu_entries(selected: usize) -> Vec<Entry<Message>> {
    let suffix = match selected {
        0 | 1 => String::new(),
        count => format!(" ({count} tracks)"),
    };

    vec![
        Entry::button(format!("Play{suffix}"), Message::PlaySelection),
        Entry::button(format!("Play next{suffix}"), Message::QueueSelectionNext),
        Entry::button(format!("Add to queue{suffix}"), Message::QueueSelection),
        Entry::Separator,
        Entry::button("Select all", Message::SelectAll),
        Entry::button("Clear selection", Message::ClearSelection),
    ]
}

fn empty_library<'a>() -> Element<'a, Message> {
    let prompt = column![
        text("No library").size(18),
        button(text("Set directory").size(14))
            .on_press(Message::SelectFolder)
            .padding(PAD * 2.0),
    ]
    .spacing(PAD * 4.0)
    .align_x(iced::Center);

    container(prompt)
        .padding(PAD * 4.0)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn no_results<'a>(query: &str) -> Element<'a, Message> {
    let label = text(format!("No results for \u{201c}{}\u{201d}", query.trim()))
        .size(14)
        .style(|theme: &Theme| text::Style {
            color: Some(
                theme
                    .extended_palette()
                    .background
                    .base
                    .text
                    .scale_alpha(0.6),
            ),
        });

    container(label)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(selected: usize) -> Vec<String> {
        menu_entries(selected)
            .into_iter()
            .filter_map(|entry| match entry {
                Entry::Button { label, .. } => Some(label),
                Entry::Separator => None,
            })
            .collect()
    }

    #[test]
    fn a_single_track_menu_does_not_announce_a_count() {
        for selected in [0, 1] {
            assert!(
                labels(selected)
                    .iter()
                    .all(|label| !label.contains("tracks")),
                "a {selected}-track menu counted its tracks"
            );
        }
    }

    #[test]
    fn a_multi_track_menu_says_how_many_it_will_act_on() {
        let labels = labels(3);
        assert!(
            labels.iter().any(|label| label.contains("(3 tracks)")),
            "{labels:?}"
        );
    }

    #[test]
    fn the_menu_offers_the_three_playback_actions() {
        let labels = labels(1);
        assert!(labels.iter().any(|label| label.starts_with("Play")));
        assert!(labels.iter().any(|label| label.starts_with("Play next")));
        assert!(labels.iter().any(|label| label.starts_with("Add to queue")));
    }
}
