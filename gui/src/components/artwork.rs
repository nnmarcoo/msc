use iced::widget::image::Handle as ImageHandle;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{Image, container, svg};
use iced::{ContentFit, Element, Length};
use msc_core::Player;

use crate::app::Message;

pub fn view<'a>(player: &Player) -> Element<'a, Message> {
    let current_track = player.clone_current_track();
    let art_cache = player.art();

    if let Some(track) = current_track {
        if let Some(rgba_image) = art_cache.get(&track) {
            let artwork = Image::new(ImageHandle::from_rgba(
                rgba_image.width,
                rgba_image.height,
                (*rgba_image.data).clone(),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(ContentFit::Contain);

            return container(artwork)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .padding(10)
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
