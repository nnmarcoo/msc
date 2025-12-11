use iced::widget::{column, container, scrollable, text};
use iced::{Element, Length};
use msc_core::Player;

use crate::app::Message;

const MAX_DISPLAY: usize = 100;

pub fn view<'a>(player: &Player) -> Element<'a, Message> {
    let queue = player.queue();
    let library = player.library();
    let current_hash = queue.current_id();

    let mut track_list = column![].spacing(2);

    if let Some(current_id) = current_hash {
        if let Some(track) = library.track_from_id(current_id) {
            track_list = track_list.push(
                container(
                    text(format!(
                        "{} - {}",
                        track.metadata.title_or_default(),
                        track.metadata.artist_or_default()
                    ))
                    .size(14),
                )
                .padding(8)
                .width(Length::Fill)
                .style(|theme: &iced::Theme| container::Style {
                    background: Some(theme.extended_palette().primary.weak.color.into()),
                    ..Default::default()
                }),
            );
        }
    }

    let upcoming = queue.upcoming();
    let total_upcoming = upcoming.len();

    for (idx, track_id) in upcoming.iter().enumerate() {
        if idx >= MAX_DISPLAY {
            track_list = track_list.push(
                container(
                    text(format!(
                        "... and {} more tracks",
                        total_upcoming - MAX_DISPLAY
                    ))
                    .size(12),
                )
                .padding(8)
                .width(Length::Fill),
            );
            break;
        }

        if let Some(track) = library.track_from_id(*track_id) {
            track_list = track_list.push(
                container(
                    text(format!(
                        "{} - {}",
                        track.metadata.title_or_default(),
                        track.metadata.artist_or_default()
                    ))
                    .size(14),
                )
                .padding(8)
                .width(Length::Fill),
            );
        }
    }

    scrollable(track_list).height(Length::Fill).into()
}
