use iced::alignment::Vertical;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{
    button, column, container, image, mouse_area, responsive, row, scrollable, stack, svg, text,
    text_input,
};
use iced::{Color, Element, Length, Radians, Theme};
use msc_core::{Player, Track};
use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::app::Message;
use crate::art_cache::ArtCache;
use crate::components::context_menu::{MenuElement, context_menu};
use crate::formatters;
use crate::image_processing::Colors;
use crate::pane_view::{PaneView, ViewContext};
use crate::styles::svg_style;

type ArtKeys = HashMap<i64, (i64, PathBuf)>;

const DEBOUNCE_TICKS: u32 = 3;
const PANEL_ART_PADDING: f32 = 8.0;

#[derive(Debug, Clone, PartialEq)]
pub enum ExpandedItem {
    Album(String, Option<String>),
    Playlist(i64),
}

#[derive(Debug, Clone)]
pub enum CollectionsMessage {
    ToggleNewPlaylistInput,
    NameChanged(String),
    Confirm(String),
    DeletePlaylist(i64),
    PlayPlaylist(i64),
    QueuePlaylistNext(i64),
    QueuePlaylistBack(i64),
    ToggleAlbum(String, Option<String>),
    TogglePlaylist(i64),
}

#[derive(Debug, Clone)]
pub struct CollectionsPane {
    pub(crate) album_art_keys: ArtKeys,
    pub(crate) playlist_art_keys: ArtKeys,
    albums_initialized: bool,
    pub(crate) playlists_initialized: bool,
    thumbnail_size: Cell<u32>,
    panel_art_size: Cell<u32>,
    stable_size: u32,
    stable_ticks: u32,
    pub(crate) creating_playlist: bool,
    pub(crate) new_playlist_name: String,
    pub(crate) expanded: Option<ExpandedItem>,
    pub(crate) expanded_tracks: Vec<Track>,
    pub(crate) expanded_cover: Option<(i64, PathBuf)>,
}

impl CollectionsPane {
    pub fn new() -> Self {
        Self {
            album_art_keys: HashMap::new(),
            playlist_art_keys: HashMap::new(),
            albums_initialized: false,
            playlists_initialized: false,
            thumbnail_size: Cell::new(0),
            panel_art_size: Cell::new(0),
            stable_size: 0,
            stable_ticks: 0,
            creating_playlist: false,
            new_playlist_name: String::new(),
            expanded: None,
            expanded_tracks: Vec::new(),
            expanded_cover: None,
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
                let panel_size = self.panel_art_size.get();
                if panel_size > 0 {
                    if let Some((tid, path)) = &self.expanded_cover {
                        art.get_or_queue(*tid, path, panel_size, panel_size);
                    }
                }
            }
        }
    }

    fn invalidate_cache(&mut self) {
        self.album_art_keys.clear();
        self.playlist_art_keys.clear();
        self.albums_initialized = false;
        self.playlists_initialized = false;
        self.expanded = None;
        self.expanded_tracks.clear();
        self.expanded_cover = None;
    }

    fn view<'a>(&'a self, ctx: ViewContext<'a>) -> Element<'a, Message> {
        let art = ctx.art;
        let albums = ctx.cached_albums.borrow().clone().unwrap_or_default();
        let playlists = ctx.cached_playlists.borrow().clone().unwrap_or_default();
        let creating_playlist = self.creating_playlist;
        let new_playlist_name = self.new_playlist_name.as_str();
        let hovered_card = ctx.hovered_card;

        let album_art_keys = &self.album_art_keys;
        let playlist_art_keys = &self.playlist_art_keys;
        let expanded = &self.expanded;
        let expanded_tracks = &self.expanded_tracks;
        let expanded_cover = &self.expanded_cover;

        responsive(move |size| {
            const MIN_CARD_WIDTH: f32 = 150.0;
            const EDGE_PADDING: f32 = 15.0;
            const GAP: f32 = 15.0;

            let available = size.width - EDGE_PADDING * 2.0;
            let cols = ((available + GAP) / (MIN_CARD_WIDTH + GAP))
                .floor()
                .max(1.0) as usize;
            let card_size = if cols > 1 {
                ((available - GAP * (cols - 1) as f32) / cols as f32).floor()
            } else {
                available.floor()
            };

            let thumb_px = card_size.round() as u32;
            self.thumbnail_size.set(thumb_px);

            let panel_height = 2.0 * card_size + GAP;
            let panel_px = (panel_height - PANEL_ART_PADDING * 2.0).round() as u32;
            self.panel_art_size.set(panel_px);

            if albums.is_empty() && playlists.is_empty() && !creating_playlist {
                return container(text("No albums or playlists").size(18).style(
                    |theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.base.text),
                    },
                ))
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
                        let colors = track_id.and_then(|id| {
                            art.get(id, thumb_px, thumb_px)
                                .or_else(|| art.get_any(id))
                                .map(|e| e.colors)
                        });
                        let art_el = art_card(art, track_id, thumb_px, card_size);
                        let album_name = album.name.clone();
                        let artist = album.artist.clone();
                        let is_hovered = hovered_card == Some((true, album.id));

                        let card = card_with_overlay(
                            art_el,
                            card_size,
                            is_hovered,
                            colors,
                            Message::PlayAlbum(album_name.clone(), artist.clone()),
                            Message::QueueAlbumBack(album_name.clone(), artist.clone()),
                            Message::Collections(CollectionsMessage::ToggleAlbum(
                                album_name.clone(),
                                artist.clone(),
                            )),
                            Message::CardHovered(true, album.id),
                        );

                        album_row = album_row.push(context_menu(
                            card,
                            vec![
                                MenuElement::button(
                                    "Play",
                                    Message::PlayAlbum(album_name.clone(), artist.clone()),
                                ),
                                MenuElement::button(
                                    "Queue next",
                                    Message::QueueAlbumNext(album_name.clone(), artist.clone()),
                                ),
                                MenuElement::button(
                                    "Add to queue",
                                    Message::QueueAlbumBack(album_name, artist),
                                ),
                            ],
                        ));
                    }

                    albums_section = albums_section.push(album_row);

                    if let Some(ExpandedItem::Album(ref name, ref artist)) = *expanded {
                        if chunk.iter().any(|a| &a.name == name) {
                            let cover_tid = expanded_cover.as_ref().map(|(tid, _)| *tid);
                            albums_section = albums_section.push(expanded_panel(
                                expanded_tracks,
                                panel_height,
                                art,
                                cover_tid,
                                panel_px,
                                name.clone(),
                                artist.clone(),
                                Message::PlayAlbum(name.clone(), artist.clone()),
                            ));
                        }
                    }
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
                        let colors = track_id.and_then(|id| {
                            art.get(id, thumb_px, thumb_px)
                                .or_else(|| art.get_any(id))
                                .map(|e| e.colors)
                        });
                        let artwork_el = art_card(art, track_id, thumb_px, card_size);
                        let pid = playlist.id;
                        let is_hovered = hovered_card == Some((false, pid));

                        let card = card_with_overlay(
                            artwork_el,
                            card_size,
                            is_hovered,
                            colors,
                            Message::Collections(CollectionsMessage::PlayPlaylist(pid)),
                            Message::Collections(CollectionsMessage::QueuePlaylistBack(pid)),
                            Message::Collections(CollectionsMessage::TogglePlaylist(pid)),
                            Message::CardHovered(false, pid),
                        );

                        playlist_row = playlist_row.push(context_menu(
                            card,
                            vec![
                                MenuElement::button(
                                    "Play",
                                    Message::Collections(CollectionsMessage::PlayPlaylist(pid)),
                                ),
                                MenuElement::button(
                                    "Queue next",
                                    Message::Collections(CollectionsMessage::QueuePlaylistNext(
                                        pid,
                                    )),
                                ),
                                MenuElement::button(
                                    "Add to queue",
                                    Message::Collections(CollectionsMessage::QueuePlaylistBack(
                                        pid,
                                    )),
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

                    if let Some(ExpandedItem::Playlist(pid)) = *expanded {
                        if let Some(pl) = chunk.iter().find(|p| p.id == pid) {
                            let cover_tid = expanded_cover.as_ref().map(|(tid, _)| *tid);
                            playlists_section = playlists_section.push(expanded_panel(
                                expanded_tracks,
                                panel_height,
                                art,
                                cover_tid,
                                panel_px,
                                pl.name.clone(),
                                None,
                                Message::Collections(CollectionsMessage::PlayPlaylist(pid)),
                            ));
                        }
                    }
                }
            } else if !creating_playlist {
                playlists_section =
                    playlists_section.push(text("No playlists").size(13).style(|theme: &Theme| {
                        text::Style {
                            color: Some(theme.extended_palette().background.strong.text),
                        }
                    }));
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

fn expanded_panel<'a>(
    tracks: &'a [Track],
    panel_height: f32,
    art: &'a ArtCache,
    cover_track_id: Option<i64>,
    panel_px: u32,
    title: String,
    artist: Option<String>,
    play_msg: Message,
) -> Element<'a, Message> {
    let art_display_size = panel_height - PANEL_ART_PADDING * 2.0;

    let entry =
        cover_track_id.and_then(|id| art.get(id, panel_px, panel_px).or_else(|| art.get_any(id)));

    let cover_color: Option<[u8; 3]> = entry.map(|e| e.colors.background);

    let panel_style = move |theme: &Theme| {
        let palette = theme.extended_palette();
        let bg_color = palette.background.weak.color;
        let background = match cover_color {
            Some([r, g, b]) => {
                let left = Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
                iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(Radians(std::f32::consts::FRAC_PI_2))
                        .add_stop(0.0, left)
                        .add_stop(1.0, bg_color),
                ))
            }
            None => iced::Background::Color(bg_color),
        };
        container::Style {
            background: Some(background),
            border: iced::Border {
                color: palette.background.strong.color,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        }
    };

    let cover: Element<'a, Message> = match entry {
        Some(entry) => container(
            image(entry.handle.clone())
                .width(Length::Fixed(art_display_size))
                .height(Length::Fixed(art_display_size))
                .content_fit(iced::ContentFit::Cover),
        )
        .padding(PANEL_ART_PADDING)
        .width(Length::Fixed(panel_height))
        .height(Length::Fixed(panel_height))
        .into(),
        None => container(placeholder_artwork(art_display_size))
            .padding(PANEL_ART_PADDING)
            .width(Length::Fixed(panel_height))
            .height(Length::Fixed(panel_height))
            .into(),
    };

    if tracks.is_empty() {
        return container(row![
            cover,
            container(
                text("No tracks")
                    .size(12)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().background.strong.text),
                    })
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill),
        ])
        .width(Length::Fill)
        .height(Length::Fixed(panel_height))
        .style(panel_style)
        .into();
    }

    let muted = |theme: &Theme| text::Style {
        color: Some(theme.extended_palette().background.strong.text),
    };

    let separator = || {
        container(iced::widget::Space::new())
            .height(Length::Fixed(1.0))
            .width(Length::Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                ..Default::default()
            })
    };

    let total_secs: u32 = tracks.iter().map(|t| t.duration() as u32).sum();
    let total_duration = {
        let h = total_secs / 3600;
        let m = (total_secs % 3600) / 60;
        let s = total_secs % 60;
        if h > 0 {
            format!("{h}:{m:02}:{s:02}")
        } else {
            format!("{m}:{s:02}")
        }
    };

    let play_all = container(
        row![
            crate::widgets::canvas_button::canvas_button(
                svg(SvgHandle::from_memory(include_bytes!(
                    "../../../assets/icons/play.svg"
                )))
                .style(svg_style),
            )
            .width(28)
            .height(28)
            .on_press(play_msg),
            text(title).size(14).font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            }),
            if let Some(a) = artist {
                Element::from(text(a).size(13).style(muted))
            } else {
                Element::from(iced::widget::Space::new().width(0))
            },
            iced::widget::Space::new().width(Length::Fill),
            text(total_duration)
                .size(11)
                .style(muted)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
    )
    .padding([6, 10]);

    let mut track_col = column![].spacing(0);
    let mut first = true;

    for (i, track) in tracks.iter().enumerate() {
        if let Some(tid) = track.id() {
            if !first {
                track_col = track_col.push(separator());
            }
            first = false;

            let num = track
                .track_number()
                .map(|n| n.to_string())
                .unwrap_or_else(|| (i + 1).to_string());
            let title = track.title().unwrap_or("-").to_string();
            let duration = formatters::format_duration(track.duration());

            let track_row = context_menu(
                button(
                    row![
                        text(num)
                            .size(11)
                            .align_x(iced::alignment::Horizontal::Right)
                            .style(muted)
                            .width(Length::Fixed(24.0)),
                        text(title).size(13).width(Length::Fill),
                        text(duration)
                            .size(11)
                            .align_x(iced::alignment::Horizontal::Right)
                            .style(muted)
                            .width(Length::Fixed(46.0)),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
                )
                .padding([7, 12])
                .width(Length::Fill)
                .style(|theme: &Theme, status| {
                    let palette = theme.extended_palette();
                    button::Style {
                        background: match status {
                            button::Status::Hovered | button::Status::Pressed => Some(
                                Color {
                                    a: 0.15,
                                    ..palette.primary.weak.color
                                }
                                .into(),
                            ),
                            _ => None,
                        },
                        text_color: palette.background.base.text,
                        ..Default::default()
                    }
                })
                .on_press(Message::PlayTrack(tid)),
                vec![
                    MenuElement::button("Play", Message::PlayTrack(tid)),
                    MenuElement::button("Queue next", Message::QueueFront(tid)),
                    MenuElement::button("Queue", Message::QueueBack(tid)),
                ],
            );

            track_col = track_col.push(track_row);
        }
    }

    let track_list = scrollable(track_col)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(0).scroller_width(0),
        ));

    let right = column![play_all, separator(), track_list].spacing(0);

    container(row![cover, right])
        .width(Length::Fill)
        .height(Length::Fixed(panel_height))
        .style(panel_style)
        .into()
}

fn card_with_overlay<'a>(
    art: Element<'a, Message>,
    card_size: f32,
    is_hovered: bool,
    colors: Option<Colors>,
    play_msg: Message,
    queue_msg: Message,
    toggle_msg: Message,
    hover_msg: Message,
) -> Element<'a, Message> {
    let art_button = button(art)
        .padding(0)
        .width(Length::Fixed(card_size))
        .height(Length::Fixed(card_size))
        .style(|_, _| button::Style::default())
        .on_press(toggle_msg);

    let overlay: Element<'a, Message> = if is_hovered {
        let (bar_color, icon_color) = match colors {
            Some(c) => {
                let bg = Color::from_rgb8(c.background[0], c.background[1], c.background[2]);
                let inv = Color::from_rgb(1.0 - bg.r, 1.0 - bg.g, 1.0 - bg.b);
                (bg, inv)
            }
            None => (Color::BLACK, Color::WHITE),
        };
        let icon_style = move |_: &Theme, _: iced::widget::svg::Status| iced::widget::svg::Style {
            color: Some(Color::WHITE),
        };

        column![
            iced::widget::Space::new().height(Length::Fill),
            container(
                row![
                    crate::widgets::canvas_button::canvas_button(
                        svg(SvgHandle::from_memory(include_bytes!(
                            "../../../assets/icons/play.svg"
                        )))
                        .style(icon_style),
                    )
                    .width(20)
                    .height(20)
                    .on_press(play_msg),
                    crate::widgets::canvas_button::canvas_button(
                        svg(SvgHandle::from_memory(include_bytes!(
                            "../../../assets/icons/queue_add.svg"
                        )))
                        .style(icon_style),
                    )
                    .width(20)
                    .height(20)
                    .on_press(queue_msg),
                ]
                .spacing(6),
            )
            .height(Length::Fixed(card_size))
            .padding(5)
            .center_x(card_size)
            .align_bottom(card_size)
            .style(move |_: &Theme| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(Radians(std::f32::consts::PI))
                        .add_stop(0.0, Color::TRANSPARENT)
                        .add_stop(1.0, Color::BLACK),
                ))),
                ..Default::default()
            }),
        ]
        .width(Length::Fixed(card_size))
        .height(Length::Fixed(card_size))
        .into()
    } else {
        iced::widget::Space::new()
            .width(Length::Fixed(card_size))
            .height(Length::Fixed(card_size))
            .into()
    };

    mouse_area(
        stack![art_button, overlay]
            .width(Length::Fixed(card_size))
            .height(Length::Fixed(card_size)),
    )
    .on_enter(hover_msg)
    .on_exit(Message::CardUnhovered)
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
