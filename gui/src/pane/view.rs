//! Renders one pane: its content, plus the edit-mode header.
//!
//! The match on [`PaneKind`] is exhaustive, so a new kind cannot be added
//! without the compiler pointing here.

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};
use verse_core::{Library, Player};

use crate::app::Message as AppMessage;
use crate::components::now_playing;
use crate::layout::{Axis, PaneId};
use crate::pane::{PaneKind, PaneState, PaneStates, library, queue};
use crate::styles::{self, LABEL_FONT_SIZE, PAD};

pub fn view<'a>(
    id: PaneId,
    kind: PaneKind,
    states: &'a PaneStates,
    library: &'a Library,
    player: &Player,
    edit_mode: bool,
) -> Element<'a, AppMessage> {
    let content = content(id, kind, states, library, player);

    if edit_mode {
        column![header(id, kind), content].into()
    } else {
        content
    }
}

fn content<'a>(
    id: PaneId,
    kind: PaneKind,
    states: &'a PaneStates,
    library: &'a Library,
    player: &Player,
) -> Element<'a, AppMessage> {
    match (kind, states.get(id)) {
        (PaneKind::Library, Some(PaneState::Library(state))) => {
            library::view(id, state, library, player)
        }
        (PaneKind::Queue, Some(PaneState::Queue(state))) => {
            queue::view(id, state, library, player)
        }
        (PaneKind::NowPlaying, _) => now_playing::view(library, player),
        (PaneKind::Empty, _) => placeholder("Empty"),
        (kind, _) => placeholder(kind.title()),
    }
}

fn header<'a>(id: PaneId, kind: PaneKind) -> Element<'a, AppMessage> {
    let mut kinds = row![].spacing(2);
    for candidate in PaneKind::ALL {
        if candidate == kind {
            continue;
        }
        kinds = kinds.push(
            button(text(candidate.title()).size(LABEL_FONT_SIZE))
                .on_press(AppMessage::SetPaneKind(id, candidate))
                .padding([2.0, PAD])
                .style(styles::icon_button_style),
        );
    }

    container(
        row![
            text(kind.title()).size(LABEL_FONT_SIZE).width(Length::Fill),
            kinds,
            button(text("split ↔").size(LABEL_FONT_SIZE))
                .on_press(AppMessage::SplitPane(id, Axis::Vertical))
                .padding([2.0, PAD])
                .style(styles::icon_button_style),
            button(text("split ↕").size(LABEL_FONT_SIZE))
                .on_press(AppMessage::SplitPane(id, Axis::Horizontal))
                .padding([2.0, PAD])
                .style(styles::icon_button_style),
            button(text("×").size(LABEL_FONT_SIZE))
                .on_press(AppMessage::ClosePane(id))
                .padding([2.0, PAD])
                .style(styles::icon_button_style),
        ]
        .spacing(PAD)
        .align_y(iced::Alignment::Center),
    )
    .padding([PAD / 2.0, PAD])
    .width(Length::Fill)
    .style(styles::bar_style)
    .into()
}

fn placeholder<'a>(label: &'a str) -> Element<'a, AppMessage> {
    container(text(label).size(LABEL_FONT_SIZE))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
