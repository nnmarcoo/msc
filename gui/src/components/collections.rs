use blake3::Hash;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length, Theme};
use msc_core::Player;

use crate::app::Message;

pub fn view<'a>(player: &Player, _hovered_track: &Option<Hash>) -> Element<'a, Message> {
    let library = player.library();

    // this seems dumb
    let mut collections: Vec<_> = library
        .collections
        .iter()
        .map(|entry| entry.value().clone())
        .collect();

    collections.sort_by(|a, b| a.name().cmp(b.name()));

    if collections.is_empty() {
        return container(
            text("No collections")
                .size(18)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.extended_palette().background.base.text),
                }),
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|theme: &Theme| container::Style {
            text_color: Some(theme.extended_palette().background.base.text),
            ..Default::default()
        })
        .into();
    }

    let header = container(
        row![
            container(
                text("Collection")
                    .size(12)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.strong.text),
                    })
            )
            .width(Length::FillPortion(3)),
            container(text("Artist").size(12).style(|theme: &Theme| text::Style {
                color: Some(theme.extended_palette().background.strong.text),
            }))
            .width(Length::FillPortion(2)),
            container(text("Tracks").size(12).style(|theme: &Theme| text::Style {
                color: Some(theme.extended_palette().background.strong.text),
            }))
            .width(Length::Fixed(80.0)),
        ]
        .spacing(10),
    )
    .padding(10)
    .width(Length::Fill)
    .style(|theme: &Theme| container::Style {
        text_color: Some(theme.extended_palette().background.strong.text),
        background: Some(theme.extended_palette().background.strong.color.into()),
        ..Default::default()
    });

    let mut collection_list = column![].spacing(0);

    for collection in collections {
        let collection_id = collection.id();
        let collection_name = collection.name().to_string();
        let artist_name = collection.artist().unwrap_or("-").to_string();
        let track_count = collection.tracks().len();

        let collection_row = container(
            button(
                row![
                    container(text(collection_name).size(12)).width(Length::FillPortion(3)),
                    container(text(artist_name).size(12)).width(Length::FillPortion(2)),
                    container(text(track_count.to_string()).size(12)).width(Length::Fixed(80.0)),
                ]
                .spacing(10),
            )
            .padding(10)
            .width(Length::Fill)
            .style(|theme: &Theme, status| {
                let palette = theme.extended_palette();
                let appearance = button::Style {
                    background: Some(match status {
                        button::Status::Hovered => palette.primary.weak.color.into(),
                        button::Status::Pressed => palette.primary.base.color.into(),
                        _ => palette.background.base.color.into(),
                    }),
                    text_color: palette.background.base.text,
                    ..Default::default()
                };
                appearance
            })
            .on_press(Message::PlayCollection(collection_id)),
        )
        .width(Length::Fill);

        collection_list = collection_list.push(collection_row);
    }

    column![
        header,
        scrollable(collection_list).height(Length::Fill).direction(
            scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(0).scroller_width(0),
            )
        )
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
