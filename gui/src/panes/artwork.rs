use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{Image, container, responsive, svg};
use iced::{Color, ContentFit, Element, Length, Theme};
use verse_core::Player;
use std::cell::Cell;

use crate::app::Message;
use crate::art_cache::ArtCache;
use crate::pane_view::{PaneView, ViewContext};
use crate::styles::svg_style;

const DEBOUNCE_TICKS: u32 = 3;

#[derive(Debug, Clone)]
pub struct ArtworkPane {
    current_track_id: Option<i64>,
    display_size: Cell<(u32, u32)>,
    stable_size: (u32, u32),
    stable_ticks: u32,
}

impl ArtworkPane {
    pub fn new() -> Self {
        Self {
            current_track_id: None,
            display_size: Cell::new((0, 0)),
            stable_size: (0, 0),
            stable_ticks: 0,
        }
    }
}

impl PaneView for ArtworkPane {
    fn update(&mut self, player: &Player, art: &mut ArtCache) {
        if let Some(track) = player.clone_current_track() {
            if let Some(id) = track.id() {
                self.current_track_id = Some(id);
                let (w, h) = self.display_size.get();
                if w > 0 && h > 0 {
                    if (w, h) == self.stable_size {
                        self.stable_ticks = self.stable_ticks.saturating_add(1);
                    } else {
                        self.stable_size = (w, h);
                        self.stable_ticks = 0;
                    }
                    if self.stable_ticks >= DEBOUNCE_TICKS {
                        art.get_or_queue(id, track.path(), w, h);
                    }
                }
            }
        } else {
            self.current_track_id = None;
        }
    }

    fn view<'a>(&'a self, ctx: ViewContext<'a>) -> Element<'a, Message> {
        let art = ctx.art;
        let (w, h) = self.display_size.get();
        let entry = self
            .current_track_id
            .and_then(|id| art.get(id, w, h).or_else(|| art.get_any(id)));

        responsive(move |size| {
            self.display_size
                .set((size.width as u32, size.height as u32));

            if let Some(entry) = entry {
                let [r, g, b] = entry.colors.background;

                container(
                    container(
                        Image::new(entry.handle.clone())
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
                .style(move |_theme: &Theme| container::Style {
                    background: Some(Color::from_rgb8(r, g, b).into()),
                    ..Default::default()
                })
                .into()
            } else {
                container(
                    svg(SvgHandle::from_memory(include_bytes!(
                        "../../../assets/icons/disk.svg"
                    )))
                    .style(svg_style),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
            }
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
