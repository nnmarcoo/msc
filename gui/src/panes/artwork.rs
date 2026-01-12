use iced::widget::image::Handle as ImageHandle;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{Image, container, responsive, svg};
use iced::{Color, ContentFit, Element, Length};
use msc_core::{Player, Track};
use std::cell::RefCell;
use std::sync::Arc;

use crate::app::Message;
use crate::pane_view::PaneView;

#[derive(Debug, Clone)]
pub struct ArtworkPane {
    // track_id, size, handle ts sucks tho maybe should have 2 image handles so it doesnt flicker
    cache: RefCell<Option<(i64, u32, ImageHandle)>>,
}

impl ArtworkPane {
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(None),
        }
    }
}

impl PaneView for ArtworkPane {
    fn update(&mut self, player: &Player) {
        // Clear cache if track changes
        if let Some(track) = player.clone_current_track() {
            if let Some(track_id) = track.id() {
                let should_clear = self
                    .cache
                    .borrow()
                    .as_ref()
                    .map(|(cached_id, _, _)| *cached_id != track_id)
                    .unwrap_or(false);

                if should_clear {
                    *self.cache.borrow_mut() = None;
                }
            }
        } else {
            *self.cache.borrow_mut() = None;
        }
    }

    fn view<'a>(
        &'a self,
        player: &'a Player,
        _volume: f32,
        _hovered_track: &Option<i64>,
        _seeking_position: Option<f32>,
        _cached_tracks: &'a RefCell<Option<Vec<Track>>>,
        _cached_albums: &'a RefCell<
            Option<Vec<(i64, String, Option<String>, Option<u32>, Option<String>)>>,
        >,
    ) -> Element<'a, Message> {
        let current_track = player.clone_current_track();
        let cache = &self.cache;

        responsive(move |size| {
            if let Some(track) = &current_track {
                if let Some(track_id) = track.id() {
                    let max_dimension = (size.width.max(size.height) - 20.0) as u32;
                    let requested_size = max_dimension.clamp(64, 2048);

                    // Check if we need to update cache
                    let mut cached = cache.borrow_mut();
                    let needs_update = cached
                        .as_ref()
                        .map(|(id, size, _)| {
                            *id != track_id || (*size as i32 - requested_size as i32).abs() > 100
                        })
                        .unwrap_or(true);

                    if needs_update {
                        if let Some((image, _)) = player.artwork(track, requested_size) {
                            let handle = ImageHandle::from_rgba(
                                image.width,
                                image.height,
                                Arc::unwrap_or_clone(image.data),
                            );
                            *cached = Some((track_id, requested_size, handle));
                        }
                    }

                    // Render with cached handle
                    if let Some((id, _, handle)) = cached.as_ref() {
                        if *id == track_id {
                            let handle = handle.clone();
                            drop(cached); // Release borrow

                            if let Some((_, colors)) = player.artwork(track, requested_size) {
                                let artwork = Image::new(handle)
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .content_fit(ContentFit::Contain);

                                let [r, g, b] = colors.background;
                                let bg_color = Color::from_rgb8(r, g, b);

                                return container(artwork)
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .center_x(Length::Fill)
                                    .center_y(Length::Fill)
                                    .padding(10)
                                    .style(move |_theme| container::Style {
                                        background: Some(bg_color.into()),
                                        ..Default::default()
                                    })
                                    .into();
                            }
                        }
                    }
                }
            }

            container(svg(SvgHandle::from_memory(include_bytes!(
                "../../../assets/icons/disk.svg"
            ))))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
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
