//! The queue pane: a count-and-clear strip above a [`QueueList`].
//!
//! The list is one widget, not a `column` of per-row ones, and it owns the rows,
//! the hover, the remove control and the reorder drag. Rows as separate widgets
//! produced three cursor bugs; see [`crate::widgets::queue_list`]. This pane is
//! the strip and the routing beneath it.
//!
//! `show_history` is per-pane state, since it describes how this pane draws rather
//! than anything about a track, so two queue panes may disagree about it. Nothing
//! toggles it yet and played tracks stay hidden.
//!
//! Everything else comes from [`Context`]. A row lights up because the track it
//! holds is hovered or selected in any pane, so pointing at a library row lights
//! the same track here, and every copy of a track lights together.
//!
//! Removal and reordering go by queue position, not by track: acting on "that
//! track" would hit copies the user never pointed at. History and the playing row
//! have no position in the upcoming deque, so the widget gives them neither a
//! remove control nor a grip, which makes a wrong index unrepresentable.

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::app::Message as AppMessage;
use crate::browsing::{Context, Slot};
use crate::styles::{self, LABEL_FONT_SIZE, PAD};
use crate::widgets::context_menu::{ContextMenu, Entry};
use crate::widgets::queue_list::{Op, QueueList};

const STRIP_PAD_H: f32 = 10.0;

#[derive(Debug, Default)]
pub struct State {
    pub show_history: bool,
}

impl State {
    pub const EMPTY: Self = Self {
        show_history: false,
    };
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleHistory,
}

pub fn update(state: &mut State, message: &Message) {
    match message {
        Message::ToggleHistory => state.show_history = !state.show_history,
    }
}

pub struct Bindings {
    pub clear: AppMessage,
}

pub fn view<'a>(
    tracks: Context<'a>,
    state: &State,
    bindings: &Bindings,
) -> Element<'a, AppMessage> {
    let rows = tracks.queued(state.show_history);

    if rows.is_empty() {
        return empty();
    }

    let total: f32 = rows.iter().map(|entry| entry.track.duration()).sum();
    let count = rows
        .iter()
        .filter(|entry| entry.slot != Slot::Played)
        .count();

    let list = QueueList::new(rows, tracks, |op| match op {
        Op::Hovered(id) => AppMessage::TrackHovered(id),
        Op::Activated(id) => AppMessage::PlayTrack(id),
        Op::Removed(index) => AppMessage::RemoveFromQueue(index),
        Op::Reordered { from, to } => AppMessage::ReorderQueue { from, to },
    });

    let body = ContextMenu::new(list, menu_entries(&bindings.clear));

    column![
        strip(count, total, bindings),
        scrollable(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn menu_entries(clear: &AppMessage) -> Vec<Entry<AppMessage>> {
    vec![
        Entry::button("Shuffle queue", AppMessage::Shuffle),
        Entry::Separator,
        Entry::button("Clear queue", clear.clone()),
    ]
}

fn strip<'a>(count: usize, total: f32, bindings: &Bindings) -> Element<'a, AppMessage> {
    let label = if count == 0 {
        "Nothing queued".to_owned()
    } else {
        crate::pane::summary(count, total)
    };

    let line = row![
        text(label).size(LABEL_FONT_SIZE).style(styles::dim_text),
        Space::new().width(Length::Fill),
        button(text("Clear").size(LABEL_FONT_SIZE))
            .on_press(bindings.clear.clone())
            .padding([2.0, PAD])
            .style(styles::icon_button_style),
    ]
    .align_y(iced::Center);

    container(line)
        .padding([PAD, STRIP_PAD_H])
        .width(Length::Fill)
        .into()
}

fn empty<'a>() -> Element<'a, AppMessage> {
    container(text("Queue is empty").size(13.0).style(styles::dim_text))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_stays_hidden_until_toggled() {
        let mut state = State::default();
        assert!(!state.show_history);

        update(&mut state, &Message::ToggleHistory);
        assert!(state.show_history);
    }
}
