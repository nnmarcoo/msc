use iced::widget::image::Handle as ImageHandle;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{container, svg, Image};
use iced::{Color, ContentFit, Element, Length};
use msc_core::{Player, Track};
use std::cell::RefCell;

#[derive(Debug, Clone)]
struct CachedArtwork {
    track_id: i64,
    actual_size: u32,
    handle: ImageHandle,
    colors: [u8; 3],
}

#[derive(Debug, Clone)]
pub struct ArtworkImage {
    cache: RefCell<Option<CachedArtwork>>,
    pub requested_size: u32,
}

impl ArtworkImage {
    pub fn new(size: u32) -> Self {
        Self {
            cache: RefCell::new(None),
            requested_size: size,
        }
    }

    pub fn update(&self, player: &Player, track: Option<&Track>) {
        let Some(track) = track else {
            *self.cache.borrow_mut() = None;
            return;
        };

        let Some(track_id) = track.id() else {
            *self.cache.borrow_mut() = None;
            return;
        };

        let needs_update = self.cache.borrow().as_ref().map_or(true, |cached| {
            cached.track_id != track_id || cached.actual_size != self.requested_size
        });

        if !needs_update {
            return;
        }

        if let Some((image, colors)) = player.artwork(track, self.requested_size) {
            let actual_size = image.width.max(image.height);

            let should_update = self.cache.borrow().as_ref().map_or(true, |cached| {
                cached.track_id != track_id || actual_size == self.requested_size
            });

            if should_update {
                *self.cache.borrow_mut() = Some(CachedArtwork {
                    track_id,
                    actual_size,
                    handle: ImageHandle::from_rgba(
                        image.width,
                        image.height,
                        image.data.to_vec(),
                    ),
                    colors: colors.background,
                });
            }
        }
    }

    pub fn view<'a, Message: 'a>(&'a self) -> Element<'a, Message> {
        if let Some(cached) = self.cache.borrow().as_ref() {
            Image::new(cached.handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(ContentFit::Contain)
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
    }

    pub fn view_with_background<'a, Message: 'a>(&'a self) -> Element<'a, Message> {
        if let Some(cached) = self.cache.borrow().as_ref() {
            let [r, g, b] = cached.colors;

            container(
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
                .padding(10),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme| iced::widget::container::Style {
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
    }
}
