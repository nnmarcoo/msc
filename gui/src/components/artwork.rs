use iced::widget::image::Handle as ImageHandle;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{Image, container, svg};
use iced::{Color, ContentFit, Element, Length};
use msc_core::Player;

use crate::app::Message;

pub fn view<'a>(player: &Player) -> Element<'a, Message> {
    let current_track = player.clone_current_track();
    let art_cache = player.art();

    if let Some(track) = current_track {
        if let Some((image, colors)) = art_cache.get(&track) {
            let artwork = Image::new(ImageHandle::from_rgba(
                image.width,
                image.height,
                (*image.data).clone(),
            ))
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
    container(svg(SvgHandle::from_memory(include_bytes!(
        "../../../assets/icons/disk.svg"
    ))))
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}
