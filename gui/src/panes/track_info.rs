use iced::font::Weight;
use iced::widget::{column, container, row, text};
use iced::{Element, Font, Length, Theme};
use msc_core::{Album, Player, Track};
use std::cell::RefCell;

use crate::app::Message;
use crate::art_cache::ArtCache;
use crate::formatters;
use crate::pane_view::PaneView;

#[derive(Debug, Clone)]
pub struct TrackInfoPane;

impl TrackInfoPane {
    pub fn new() -> Self {
        Self
    }
}

impl PaneView for TrackInfoPane {
    fn update(&mut self, _player: &Player, _art: &mut ArtCache) {}

    fn view<'a>(
        &'a self,
        player: &'a Player,
        _volume: f32,
        _hovered_track: &Option<i64>,
        _seeking_position: Option<f32>,
        _cached_tracks: &'a RefCell<Option<Vec<Track>>>,
        _cached_albums: &'a RefCell<Option<Vec<Album>>>,
        _art: &'a ArtCache,
    ) -> Element<'a, Message> {
        let Some(track) = player.clone_current_track() else {
            return container(text(""))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        };

        let title_text = text(track.title().unwrap_or("-").to_string())
            .size(18)
            .font(Font {
                weight: Weight::Bold,
                ..Default::default()
            });

        let artist_text = text(track.track_artist().unwrap_or("-").to_string()).size(15);

        let album_genre_parts: Vec<_> = [track.album(), track.genre()]
            .into_iter()
            .flatten()
            .map(|s| s.to_string())
            .collect();

        let album_genre = if !album_genre_parts.is_empty() {
            text(album_genre_parts.join(" • "))
                .size(13)
                .style(secondary_style)
        } else {
            text("").size(13)
        };

        let duration_text =
            text(formatters::format_duration(track.duration())).size(13).style(secondary_style);

        let quality_parts: Vec<_> = [
            formatters::format_sample_rate(track.sample_rate()),
            formatters::format_optional_u8(track.bit_depth(), "bit"),
            formatters::format_optional_u32(track.bit_rate(), "kbps"),
        ]
        .into_iter()
        .filter(|s| s != "-")
        .collect();

        let quality_text = if !quality_parts.is_empty() {
            text(quality_parts.join(" • ")).size(13).style(secondary_style)
        } else {
            text("").size(13)
        };

        let channels_text = text(formatters::format_channels(track.channels()))
            .size(13)
            .style(secondary_style);

        let main_info = column![title_text, artist_text, album_genre].spacing(6);

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

        container(column![main_info, technical_info].spacing(10).padding(10))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn title(&self) -> &str {
        "Track Info"
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}

fn secondary_style(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().background.weak.text),
    }
}
