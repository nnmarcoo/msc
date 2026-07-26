//! The queue pane: what has played, what is playing, what is next.
//!
//! The pane is the list and nothing else: no toolbar, no count, no controls.
//! What is queued is legible from the rows themselves, so chrome above them only
//! costs height in a pane that is usually narrow and short.
//!
//! `show_history` remains as per-pane state — it is about how this pane draws
//! rather than about any track, so it belongs here and two queue panes could
//! disagree about it — but nothing toggles it yet, so played tracks stay hidden.
//! [`Message::ToggleHistory`] is the hook for whatever exposes it later; it is
//! kept because this is the only pane carrying state, and dropping it would
//! leave [`crate::pane::PaneState`] with no variant to be generic over.
//!
//! Everything else the pane draws comes from [`Context`]: the queue itself, and
//! the shared hover and selection.
//!
//! That sharing is the point of the pane. A row lights up because the track it
//! holds is hovered or selected *anywhere*, so pointing at a row in the library
//! highlights the same track here without either pane knowing the other exists.
//! A queue can hold the same track more than once, and every copy lights up
//! together: the highlight answers "where does this song sit in the queue",
//! which is a question about the song, not about one row.
//!
//! Rows stay positional where position is what matters. Removal is by index,
//! since removing "that track" would take out copies the user never pointed at,
//! and the playing row is drawn from its [`Slot`] rather than by comparing ids —
//! the same track queued twice is playing in one place only.
//!
//! That index is [`QueueRow::upcoming`], not the row's place in the list on
//! screen: the queue removes by position within its upcoming deque, which the
//! current track and any shown history are not part of. Rows without one — the
//! played and the playing — get no double-click binding at all, so there is no
//! "remove" for a track that has already gone or is playing right now.
//!
//! History is drawn dimmed above the current track, so the list reads in play
//! order from top to bottom. It is hidden by default because it grows without
//! bound while the rest of the pane is short.
//!
//! [`Message`] is this pane's own, routed through [`PaneMessage`]; the app-level
//! messages the rows emit are [`crate::app::Message`], which is why both are in
//! scope here under distinct names.

use iced::widget::{column, container, mouse_area, row, scrollable, text};
use iced::{Element, Length, Theme};

use verse_core::Track;

use crate::app::Message as AppMessage;
use crate::styles::{self, PAD};
use crate::tracks::{Context, Slot};

const ROW_TEXT_SIZE: f32 = 12.0;

#[derive(Debug, Default)]
pub struct State {
    pub show_history: bool,
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

pub fn view(tracks: Context<'_>, show_history: bool) -> Element<'_, AppMessage> {
    let rows = tracks.queued(show_history);

    if rows.is_empty() {
        return container(text("Queue is empty").size(ROW_TEXT_SIZE).style(dim_style))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    }

    let mut list = column![].spacing(1);
    for entry in &rows {
        let state = entry
            .track
            .id()
            .map(|id| tracks.row_state(id))
            .unwrap_or_default();
        list = list.push(queue_row(
            entry.upcoming,
            entry.slot,
            entry.track,
            state.hovered,
            state.selected,
        ));
    }

    scrollable(list)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .into()
}

fn queue_row(
    upcoming: Option<usize>,
    slot: Slot,
    track: &Track,
    hovered: bool,
    selected: bool,
) -> Element<'_, AppMessage> {
    let line = row![
        text(track.title().unwrap_or("\u{2014}"))
            .size(ROW_TEXT_SIZE)
            .width(Length::FillPortion(3)),
        text(track.track_artist().unwrap_or("\u{2014}"))
            .size(ROW_TEXT_SIZE)
            .width(Length::FillPortion(2)),
    ]
    .spacing(PAD)
    .align_y(iced::Center);

    let body = container(line)
        .padding([PAD / 2.0, PAD])
        .width(Length::Fill)
        .style(move |theme: &Theme| row_style(theme, slot, hovered, selected));

    let id = track.id();

    let row = mouse_area(body)
        .on_enter(AppMessage::TrackHovered(id))
        .on_exit(AppMessage::TrackHovered(None));

    match upcoming {
        Some(index) => row
            .on_double_click(AppMessage::RemoveFromQueue(index))
            .into(),
        None => row.into(),
    }
}

fn row_style(theme: &Theme, slot: Slot, hovered: bool, selected: bool) -> container::Style {
    let palette = theme.extended_palette();

    let background = if hovered {
        Some(palette.background.strong.color.scale_alpha(0.55).into())
    } else if selected {
        Some(palette.primary.base.color.scale_alpha(0.30).into())
    } else if slot == Slot::Current {
        Some(palette.primary.base.color.scale_alpha(0.18).into())
    } else {
        None
    };

    let text_color = match slot {
        Slot::Played => palette.background.base.text.scale_alpha(0.45),
        Slot::Current => palette.primary.base.color,
        Slot::Upcoming => palette.background.base.text,
    };

    container::Style {
        text_color: Some(text_color),
        background,
        border: iced::border::rounded(styles::radius()),
        ..Default::default()
    }
}

fn dim_style(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(
            theme
                .extended_palette()
                .background
                .base
                .text
                .scale_alpha(0.6),
        ),
    }
}
