use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{button, column, container, image, responsive, row, scrollable, svg, text};
use iced::{Element, Length, Theme};
use msc_core::Player;
use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::app::Message;
use crate::art_cache::ArtCache;
use crate::pane_view::{PaneView, ViewContext};
use crate::styles::svg_style;

type AlbumArtKeys = HashMap<i64, (i64, PathBuf)>;

const DEBOUNCE_TICKS: u32 = 3;

#[derive(Debug, Clone)]
pub struct CollectionsPane {
    album_art_keys: AlbumArtKeys,
    initialized: bool,
    thumbnail_size: Cell<u32>,
    stable_size: u32,
    stable_ticks: u32,
}

impl CollectionsPane {
    pub fn new() -> Self {
        Self {
            album_art_keys: HashMap::new(),
            initialized: false,
            thumbnail_size: Cell::new(0),
            stable_size: 0,
            stable_ticks: 0,
        }
    }
}

impl PaneView for CollectionsPane {
    fn update(&mut self, player: &Player, art: &mut ArtCache) {
        if !self.initialized {
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
                self.initialized = true;
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
            }
        }
    }

    fn invalidate_cache(&mut self) {
        self.album_art_keys.clear();
        self.initialized = false;
    }

    fn view<'a>(&'a self, ctx: ViewContext<'a>) -> Element<'a, Message> {
        let art = ctx.art;
        let albums = ctx.cached_albums.borrow().clone().unwrap_or_default();

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
            .into();
        }

        let art_keys = &self.album_art_keys;

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

            let mut content = column![].spacing(GAP).padding(EDGE_PADDING);

            for chunk in albums.chunks(cols) {
                let mut album_row = row![].spacing(GAP);

                for album in chunk {
                    let track_id = art_keys.get(&album.id).map(|(tid, _)| *tid);

                    let artwork_el: Element<'_, Message> = match track_id
                        .and_then(|id| art.get(id, thumb_px, thumb_px).or_else(|| art.get_any(id)))
                    {
                        Some(entry) => image(entry.handle.clone())
                            .width(Length::Fixed(card_size))
                            .height(Length::Fixed(card_size))
                            .content_fit(iced::ContentFit::Cover)
                            .into(),
                        None => placeholder_artwork(card_size),
                    };

                    let album_name = album.name.clone();
                    let artist = album.artist.clone();
                    album_row = album_row.push(
                        button(artwork_el)
                            .padding(0)
                            .on_press(Message::PlayAlbum(album_name, artist)),
                    );
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

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
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
