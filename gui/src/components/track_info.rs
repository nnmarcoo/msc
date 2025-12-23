use iced::font::Weight;
use iced::widget::{column, container, row, text};
use iced::{Element, Font, Length, Theme};
use msc_core::Player;

use crate::formatters;

pub fn view(player: &Player) -> Element<'static, crate::app::Message> {
    let current_track = player.clone_current_track();

    if let Some(track) = current_track {
        let metadata = &track.metadata;

        let title_text = text(metadata.title_or_default()).size(18).font(Font {
            weight: Weight::Bold,
            ..Default::default()
        });

        let artist_text = text(metadata.artist_or_default()).size(15);

        let album_genre_parts = vec![metadata.album.clone(), metadata.genre.clone()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let album_genre = if !album_genre_parts.is_empty() {
            text(album_genre_parts.join(" • "))
                .size(13)
                .style(secondary_style)
        } else {
            text("").size(13)
        };

        let duration_text = text(formatters::format_duration(metadata.duration))
            .size(13)
            .style(secondary_style);

        let quality_parts = vec![
            formatters::format_sample_rate(metadata.sample_rate),
            formatters::format_optional_u8(metadata.bit_depth, "bit"),
            formatters::format_optional_u32(metadata.bit_rate, "kbps"),
        ]
        .into_iter()
        .filter(|s| s != "-")
        .collect::<Vec<_>>();

        let quality_text = if !quality_parts.is_empty() {
            text(quality_parts.join(" • "))
                .size(13)
                .style(secondary_style)
        } else {
            text("").size(13)
        };

        let channels_text = if metadata.channels.is_some() {
            text(formatters::format_channels(metadata.channels))
                .size(13)
                .style(secondary_style)
        } else {
            text("").size(13)
        };

        let main_info = column![title_text, artist_text, album_genre,].spacing(6);

        let technical_info = column![
            row![
                duration_text,
                text(" • ").size(13).style(secondary_style),
                channels_text
            ]
            .spacing(0),
            quality_text,
        ]
        .spacing(6);

        let content = column![main_info, technical_info].spacing(10).padding(10);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    } else {
        container(
            text("No track playing")
                .size(14)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.extended_palette().background.weak.text),
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}

fn secondary_style(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().background.weak.text),
    }
}
