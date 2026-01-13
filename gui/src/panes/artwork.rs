use iced::widget::image::Handle as ImageHandle;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{Image, container, svg};
use iced::{Color, ContentFit, Element, Length};
use msc_core::{Player, Track};
use std::cell::RefCell;
use std::sync::Arc;

use crate::app::Message;
use crate::pane_view::PaneView;

#[derive(Debug, Clone)]
pub struct ArtworkPane {
    cache: RefCell<Option<CachedArtwork>>,
    requested_size: RefCell<u32>,
}

#[derive(Clone)]
struct CachedArtwork {
    track_id: i64,
    actual_size: u32,
    handle: ImageHandle,
    colors: [u8; 3],
}

impl std::fmt::Debug for CachedArtwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedArtwork")
            .field("track_id", &self.track_id)
            .field("actual_size", &self.actual_size)
            .finish()
    }
}

impl ArtworkPane {
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(None),
            requested_size: RefCell::new(0),
        }
    }
}

impl PaneView for ArtworkPane {
    fn update(&mut self, player: &Player) {
        let Some(track) = player.clone_current_track() else {
            *self.cache.borrow_mut() = None;
            return;
        };

        let Some(track_id) = track.id() else {
            *self.cache.borrow_mut() = None;
            return;
        };

        let requested = *self.requested_size.borrow();
        if requested == 0 {
            return;
        }

        let needs_update = self.cache.borrow().as_ref().map_or(true, |cached| {
            cached.track_id != track_id || cached.actual_size != requested
        });

        if !needs_update {
            return;
        }

        if let Some((image, colors)) = player.artwork(&track, requested) {
            let actual_size = image.width.max(image.height);

            let should_update = self.cache.borrow().as_ref().map_or(true, |cached| {
                cached.track_id != track_id || actual_size == requested
            });

            if should_update {
                *self.cache.borrow_mut() = Some(CachedArtwork {
                    track_id,
                    actual_size,
                    handle: ImageHandle::from_rgba(
                        image.width,
                        image.height,
                        Arc::unwrap_or_clone(image.data),
                    ),
                    colors: colors.background,
                });
            }
        }
    }

    fn view<'a>(
        &'a self,
        _player: &'a Player,
        _volume: f32,
        _hovered_track: &Option<i64>,
        _seeking_position: Option<f32>,
        _cached_tracks: &'a RefCell<Option<Vec<Track>>>,
        _cached_albums: &'a RefCell<
            Option<Vec<(i64, String, Option<String>, Option<u32>, Option<String>)>>,
        >,
    ) -> Element<'a, Message> {
        let requested_size = &self.requested_size;
        let cache = &self.cache;

        iced::widget::responsive(move |size| {
            *requested_size.borrow_mut() =
                (size.width.max(size.height).ceil() as u32).clamp(64, 2048);

            if let Some(cached) = cache.borrow().as_ref() {
                let [r, g, b] = cached.colors;

                container(
                    Image::new(cached.handle.clone())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .content_fit(ContentFit::Contain),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .padding(10)
                .style(move |_theme| container::Style {
                    background: Some(Color::from_rgb8(r, g, b).into()),
                    ..Default::default()
                })
                .into()
            } else {
                container(svg(SvgHandle::from_memory(include_bytes!(
                    "../../../assets/icons/disk.svg"
                ))))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
            }
        })
        .into()
    }

    fn title(&self) -> &str {
        "Artwork"
    }

    fn clone_box(&self) -> Box<dyn PaneView> {
        Box::new(self.clone())
    }
}
