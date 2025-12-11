use iced::widget::image::Handle;
use iced::widget::{container, Image};
use iced::{ContentFit, Element, Length};
use msc_core::Player;

use crate::layout::Message;

pub fn view<'a>(player: &Player) -> Element<'a, Message> {
    let current_track = player.current_track();
    let art_cache = player.art();

    if let Some(track) = current_track {
        if let Some(rgba_image) = art_cache.get(&track) {
            let artwork = Image::new(Handle::from_rgba(
                rgba_image.width,
                rgba_image.height,
                rgba_image.data.as_ref().clone(),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(ContentFit::Contain);

            return container(artwork)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }
    }

    container("")
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
