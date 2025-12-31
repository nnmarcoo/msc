use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{button, column, container, image, responsive, row, scrollable, svg, text};
use iced::{Element, Length, Theme};
use msc_core::Player;

use crate::app::Message;

pub fn view<'a>(
    player: &'a Player,
    cached_albums: Vec<(i64, String, Option<String>, Option<u32>, Option<String>)>,
) -> Element<'a, Message> {
    let library = player.library();

    let albums = cached_albums;

    if albums.is_empty() {
        return container(
            text("No albums")
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

    let album_cards: Vec<_> = albums
        .iter()
        .map(|(album_id, album_name, artist, _year, sample_track_path)| {
            (
                *album_id,
                album_name.clone(),
                artist.clone(),
                sample_track_path.clone(),
            )
        })
        .collect();

    responsive(move |size| {
        const MIN_CARD_WIDTH: f32 = 150.0;
        const EDGE_PADDING: f32 = 15.0;
        const MIN_GAP: f32 = 15.0;

        let available_width = size.width - (EDGE_PADDING * 2.0);

        let albums_per_row = ((available_width + MIN_GAP) / (MIN_CARD_WIDTH + MIN_GAP))
            .floor()
            .max(1.0) as usize;

        let gap = MIN_GAP;
        let card_size = if albums_per_row > 1 {
            (available_width - (gap * (albums_per_row - 1) as f32)) / albums_per_row as f32
        } else {
            available_width
        };

        let mut content = column![].spacing(gap).padding(EDGE_PADDING);

        for chunk in album_cards.chunks(albums_per_row) {
            let mut album_row = row![].spacing(gap);

            for (album_id, album_name, artist, sample_track_path) in chunk.iter() {
                let album_element = create_album_card(
                    player,
                    library,
                    *album_id,
                    album_name.clone(),
                    artist.clone(),
                    sample_track_path.clone(),
                    card_size,
                );
                album_row = album_row.push(album_element);
            }

            content = content.push(album_row);
        }

        container(scrollable(content).width(Length::Fill).direction(
            scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(0).scroller_width(0),
            ),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    })
    .into()
}

fn create_album_card<'a>(
    player: &'a Player,
    library: &msc_core::Library,
    _album_id: i64,
    album_name: String,
    artist: Option<String>,
    sample_track_path: Option<String>,
    card_size: f32,
) -> Element<'a, Message> {
    let artwork_size = card_size.max(64.0).min(400.0) as u32;

    let artwork_element: Element<'a, Message> = if let Some(track_path) = sample_track_path {
        if let Ok(Some(track)) = player.library().query_track_from_path(&track_path) {
            if let Some((rgba_image, _colors)) = library.artwork(&track, artwork_size) {
                let width = rgba_image.width;
                let height = rgba_image.height;
                let raw_data = (*rgba_image.data).clone();

                let handle = image::Handle::from_rgba(width, height, raw_data);
                image(handle)
                    .width(Length::Fixed(card_size))
                    .height(Length::Fixed(card_size))
                    .into()
            } else {
                create_placeholder_artwork(card_size)
            }
        } else {
            create_placeholder_artwork(card_size)
        }
    } else {
        create_placeholder_artwork(card_size)
    };

    let album_name_display = album_name.clone();
    let artist_display = artist
        .clone()
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album_text = text(album_name_display)
        .size(13)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.extended_palette().background.base.text),
        });

    let artist_text = text(artist_display)
        .size(11)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.extended_palette().background.weak.text),
        });

    let info = column![album_text, artist_text]
        .spacing(3)
        .width(Length::Fixed(card_size));

    let card_content = column![artwork_element, info].spacing(8);

    button(card_content)
        .padding(0)
        .style(|theme: &Theme, status| {
            let palette = theme.extended_palette();
            button::Style {
                background: Some(match status {
                    button::Status::Hovered => palette.primary.weak.color.into(),
                    button::Status::Pressed => palette.primary.base.color.into(),
                    _ => iced::Color::TRANSPARENT.into(),
                }),
                ..Default::default()
            }
        })
        .on_press(Message::PlayAlbum(album_name, artist))
        .into()
}

fn create_placeholder_artwork<'a>(card_size: f32) -> Element<'a, Message> {
    let icon_size = (card_size * 0.4).max(32.0).min(128.0);

    container(
        svg(SvgHandle::from_memory(include_bytes!(
            "../../../assets/icons/disk.svg"
        )))
        .width(Length::Fixed(icon_size))
        .height(Length::Fixed(icon_size)),
    )
    .width(Length::Fixed(card_size))
    .height(Length::Fixed(card_size))
    .center_x(card_size)
    .center_y(card_size)
    .style(|theme: &Theme| container::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        ..Default::default()
    })
    .into()
}
