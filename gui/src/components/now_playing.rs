//! Now-playing pane: the current track's metadata.

use iced::widget::{column, container, text};
use iced::{Element, Length};
use verse_core::{Library, Player};

use crate::app::Message;
use crate::components::format_time;
use crate::styles::{LABEL_FONT_SIZE, PAD, ROW_FONT_SIZE};

pub fn view<'a>(library: &'a Library, player: &Player) -> Element<'a, Message> {
    let Some(track) = player.current_track(library) else {
        return container(text("Nothing playing").size(LABEL_FONT_SIZE))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    };

    let mut details = column![
        text(track.title().unwrap_or("Unknown")).size(20),
        text(track.track_artist().unwrap_or("Unknown Artist")).size(ROW_FONT_SIZE),
        text(track.album().unwrap_or("—")).size(ROW_FONT_SIZE),
    ]
    .spacing(PAD)
    .align_x(iced::Alignment::Center);

    if let Some(stars) = track.rating() {
        details = details.push(text("★".repeat(stars as usize)).size(ROW_FONT_SIZE));
    }

    details = details.push(
        text(format!(
            "{} / {}",
            format_time(player.position()),
            format_time(f64::from(track.duration()))
        ))
        .size(LABEL_FONT_SIZE),
    );

    container(details)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
