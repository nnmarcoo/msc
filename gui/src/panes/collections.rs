use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{
    button, column, container, image, responsive, row, scrollable, svg, text, text_input,
};
use iced::{Element, Length, Theme};
use msc_core::Player;
use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::app::Message;
use crate::art_cache::ArtCache;
use crate::components::context_menu::{MenuElement, context_menu};
use crate::pane_view::{PaneView, ViewContext};
use crate::styles::svg_style;

type ArtKeys = HashMap<i64, (i64, PathBuf)>;

const DEBOUNCE_TICKS: u32 = 3;

#[derive(Debug, Clone)]
pub enum CollectionsMessage {
    ToggleNewPlaylistInput,
    NameChanged(String),
    Confirm(String),
    Cancel,
    DeletePlaylist(i64),
    PlayPlaylist(i64),
}

#[derive(Debug, Clone)]
pub struct CollectionsPane {
    album_art_keys: ArtKeys,
    pub(crate) playlist_art_keys: ArtKeys,
    albums_initialized: bool,
    pub(crate) playlists_initialized: bool,
    thumbnail_size: Cell<u32>,
    stable_size: u32,
    stable_ticks: u32,
    pub(crate) creating_playlist: bool,
    pub(crate) new_playlist_name: String,
}

impl CollectionsPane {
    pub fn new() -> Self {
        Self {
            album_art_keys: HashMap::new(),
            playlist_art_keys: HashMap::new(),
            albums_initialized: false,
            playlists_initialized: false,
            thumbnail_size: Cell::new(0),
            stable_size: 0,
            stable_ticks: 0,
            creating_playlist: false,
            new_playlist_name: String::new(),
        }
    }
}

impl PaneView for CollectionsPane {
    fn update(&mut self, player: &Player, art: &mut ArtCache) {
        if !self.albums_initialized {
            if let Ok(albums) = player.query_all_albums() {
                for album in &albums {
                    if let Some(ref path_str) = album.sample_track_path {
                        if let Ok(Some(track)) = player.query_track_from_path(path_str) {
                            if let Some(tid) = track.id() {
                                self.album_art_keys
                                    .insert(album.id, (tid, track.path().to_path_buf()));
                            }
                        }
                    }
                }
                self.albums_initialized = true;
            }
        }

        if !self.playlists_initialized {
            if let Ok(playlists) = player.get_all_playlists() {
                for playlist in &playlists {
                    if let Some(tid) = playlist.cover_track_id {
                        if let Ok(Some(track)) = player.query_track_from_id(tid) {
                            self.playlist_art_keys
                                .insert(playlist.id, (tid, track.path().to_path_buf()));
                        }
                    }
                }
                self.playlists_initialized = true;
            }
        }

        let size = self.thumbnail_size.get();
        if size > 0 {
            if size == self.stable_size {
                self.stable_ticks = self.stable_ticks.saturating_add(1);
            } else {
                self.stable_size = size;
                self.stable_ticks = 0;
            }
            if self.stable_ticks >= DEBOUNCE_TICKS {
                for (_, (tid, path)) in &self.album_art_keys {
                    art.get_or_queue(*tid, path, size, size);
                }
                for (_, (tid, path)) in &self.playlist_art_keys {
                    art.get_or_queue(*tid, path, size, size);
                }
            }
        }
    }

    fn invalidate_cache(&mut self) {
        self.album_art_keys.clear();
        self.playlist_art_keys.clear();
        self.albums_initialized = false;
        self.playlists_initialized = false;
    }

    fn view<'a>(&'a self, ctx: ViewContext<'a>) -> Element<'a, Message> {
        let art = ctx.art;
        let albums = ctx.cached_albums.borrow().clone().unwrap_or_default();
        let playlists = ctx.cached_playlists.borrow().clone().unwrap_or_default();
        let creating_playlist = self.creating_playlist;
        let new_playlist_name = self.new_playlist_name.as_str();

        let album_art_keys = &self.album_art_keys;
        let playlist_art_keys = &self.playlist_art_keys;

        responsive(move |size| {
            const MIN_CARD_WIDTH: f32 = 150.0;
            const EDGE_PADDING: f32 = 15.0;
            const GAP: f32 = 15.0;

            let available = size.width - EDGE_PADDING * 2.0;
            let cols = ((available + GAP) / (MIN_CARD_WIDTH + GAP))
                .floor()
                .max(1.0) as usize;
            let card_size = if cols > 1 {
                (available - GAP * (cols - 1) as f32) / cols as f32
            } else {
                available
            };

            let thumb_px = card_size.round() as u32;
            self.thumbnail_size.set(thumb_px);

            if albums.is_empty() && playlists.is_empty() && !creating_playlist {
                return container(
                    text("No albums or playlists")
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
                .into();
            }

            let mut content = column![].spacing(GAP * 2.0).padding(EDGE_PADDING);

            if !albums.is_empty() {
                let mut albums_section = column![section_header("Albums")].spacing(GAP);

                for chunk in albums.chunks(cols) {
                    let mut album_row = row![].spacing(GAP);

                    for album in chunk {
                        let track_id = album_art_keys.get(&album.id).map(|(tid, _)| *tid);
                        let artwork_el = art_card(art, track_id, thumb_px, card_size);
                        let album_name = album.name.clone();
                        let artist = album.artist.clone();

                        album_row = album_row.push(
                            button(artwork_el)
                                .padding(0)
                                .on_press(Message::PlayAlbum(album_name, artist)),
                        );
                    }

                    albums_section = albums_section.push(album_row);
                }

                content = content.push(albums_section);
            }

            let playlists_header = row![
                section_header("Playlists"),
                iced::widget::Space::new().width(Length::Fill),
                button(text(if creating_playlist { "✕" } else { "+" }).size(14))
                    .padding([2, 8])
                    .on_press(Message::Collections(
                        CollectionsMessage::ToggleNewPlaylistInput,
                    )),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(8);

            let mut playlists_section = column![playlists_header].spacing(GAP);

            if creating_playlist {
                let input_row = row![
                    text_input("Playlist name…", new_playlist_name)
                        .on_input(|s| Message::Collections(CollectionsMessage::NameChanged(s)))
                        .on_submit(Message::Collections(CollectionsMessage::Confirm(
                            new_playlist_name.trim().to_string(),
                        )))
                        .padding(6),
                    button(text("Add").size(13))
                        .padding([6, 12])
                        .on_press(Message::Collections(CollectionsMessage::Confirm(
                            new_playlist_name.trim().to_string(),
                        ))),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center);

                playlists_section = playlists_section.push(input_row);
            }

            if !playlists.is_empty() {
                for chunk in playlists.chunks(cols) {
                    let mut playlist_row = row![].spacing(GAP);

                    for playlist in chunk {
                        let track_id = playlist_art_keys.get(&playlist.id).map(|(tid, _)| *tid);
                        let artwork_el = art_card(art, track_id, thumb_px, card_size);
                        let pid = playlist.id;

                        playlist_row = playlist_row.push(context_menu(
                            button(artwork_el)
                                .padding(0)
                                .on_press(Message::Collections(
                                    CollectionsMessage::PlayPlaylist(pid),
                                )),
                            vec![
                                MenuElement::button(
                                    "Play",
                                    Message::Collections(CollectionsMessage::PlayPlaylist(pid)),
                                ),
                                MenuElement::Separator,
                                MenuElement::button(
                                    "Delete",
                                    Message::Collections(CollectionsMessage::DeletePlaylist(pid)),
                                ),
                            ],
                        ));
                    }

                    playlists_section = playlists_section.push(playlist_row);
                }
            } else if !creating_playlist {
                playlists_section = playlists_section.push(
                    text("No playlists")
                        .size(13)
                        .style(|theme: &Theme| text::Style {
                            color: Some(theme.extended_palette().background.strong.text),
                        }),
                );
            }

            content = content.push(playlists_section);

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

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}

fn art_card<'a>(
    art: &ArtCache,
    track_id: Option<i64>,
    thumb_px: u32,
    card_size: f32,
) -> Element<'a, Message> {
    match track_id.and_then(|id| art.get(id, thumb_px, thumb_px).or_else(|| art.get_any(id))) {
        Some(entry) => image(entry.handle.clone())
            .width(Length::Fixed(card_size))
            .height(Length::Fixed(card_size))
            .content_fit(iced::ContentFit::Cover)
            .into(),
        None => placeholder_artwork(card_size),
    }
}

fn section_header<'a, Message: 'a>(label: &'a str) -> Element<'a, Message> {
    text(label)
        .size(13)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.extended_palette().background.strong.text),
        })
        .into()
}

fn placeholder_artwork<'a>(card_size: f32) -> Element<'a, Message> {
    let icon_size = (card_size * 0.4).max(32.0).min(128.0);

    container(
        svg(SvgHandle::from_memory(include_bytes!(
            "../../../assets/icons/disk.svg"
        )))
        .style(svg_style)
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
