//! Transport controls: previous, play/pause, next.
//!
//! A pane can be any size the layout gives it, so [`Metrics`] sizes the row
//! from the space available. Below the width needed for three buttons the row
//! drops to play/pause alone.

use iced::widget::svg::Handle;
use iced::widget::tooltip::Position;
use iced::widget::{button, container, responsive, row, svg, text, tooltip};
use iced::{Element, Length};

use crate::app::Message;
use crate::styles::{self, LABEL_FONT_SIZE, PAD, TOOLTIP_DELAY};

const ICON_PLAY: &[u8] = include_bytes!("../../../assets/icons/play.svg");
const ICON_PAUSE: &[u8] = include_bytes!("../../../assets/icons/pause.svg");
const ICON_NEXT: &[u8] = include_bytes!("../../../assets/icons/next.svg");
const ICON_PREVIOUS: &[u8] = include_bytes!("../../../assets/icons/previous.svg");

const ICON_MIN: f32 = 20.0;
const ICON_MAX: f32 = 48.0;
const HEIGHT_SHARE: f32 = 0.45;
const PRIMARY_SCALE: f32 = 1.4;

pub fn view<'a>(is_playing: bool) -> Element<'a, Message> {
    responsive(move |size| transport(is_playing, Metrics::pick(size))).into()
}

fn transport<'a>(is_playing: bool, metrics: Metrics) -> Element<'a, Message> {
    let (play_icon, play_label) = if is_playing {
        (ICON_PAUSE, "Pause")
    } else {
        (ICON_PLAY, "Play")
    };

    let play_pause = icon_button(
        play_icon,
        play_label,
        Message::PlayPause,
        metrics.primary_icon(),
    );

    let controls = if metrics.full {
        row![
            icon_button(ICON_PREVIOUS, "Previous", Message::Previous, metrics.icon),
            play_pause,
            icon_button(ICON_NEXT, "Next", Message::Next, metrics.icon),
        ]
    } else {
        row![play_pause]
    };

    container(controls.spacing(metrics.spacing).align_y(iced::Center))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(PAD)
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Metrics {
    icon: f32,
    spacing: f32,
    full: bool,
}

impl Metrics {
    fn pick(pane: iced::Size) -> Self {
        let icon = Self::icon_for_height(pane.height);
        let spacing = PAD * 2.0;
        Self {
            icon,
            spacing,
            full: pane.width >= Self::full_width(icon, spacing),
        }
    }

    fn icon_for_height(height: f32) -> f32 {
        (height * HEIGHT_SHARE).clamp(ICON_MIN, ICON_MAX) / PRIMARY_SCALE
    }

    fn primary_icon(self) -> f32 {
        self.icon * PRIMARY_SCALE
    }

    fn button_padding() -> f32 {
        PAD
    }

    fn full_width(icon: f32, spacing: f32) -> f32 {
        let button = |size: f32| size + Self::button_padding() * 2.0;
        button(icon) * 2.0 + button(icon * PRIMARY_SCALE) + spacing * 2.0 + PAD * 2.0
    }
}

fn icon_button<'a>(
    bytes: &'static [u8],
    label: &'a str,
    message: Message,
    size: f32,
) -> Element<'a, Message> {
    let glyph = svg(Handle::from_memory(bytes))
        .style(styles::svg_style)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size));

    let control = button(glyph)
        .on_press(message)
        .padding(Metrics::button_padding())
        .style(styles::icon_button_style);

    tooltip(
        control,
        container(text(label).size(LABEL_FONT_SIZE))
            .padding(PAD)
            .style(styles::tooltip_style),
        Position::Top,
    )
    .delay(TOOLTIP_DELAY)
    .gap(PAD)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MIN_PANE;
    use iced::Size;

    #[test]
    fn wide_pane_shows_all_three_buttons() {
        assert!(Metrics::pick(Size::new(400.0, 80.0)).full);
    }

    #[test]
    fn smallest_pane_falls_back_to_play_pause() {
        assert!(!Metrics::pick(Size::new(MIN_PANE, MIN_PANE)).full);
    }

    #[test]
    fn icon_never_leaves_the_readable_range() {
        for height in [0.0, MIN_PANE, 200.0, 2000.0] {
            let primary = Metrics::pick(Size::new(400.0, height)).primary_icon();
            assert!(
                (ICON_MIN..=ICON_MAX).contains(&primary),
                "height {height} gave primary icon {primary}"
            );
        }
    }

    #[test]
    fn buttons_fit_within_the_pane_height() {
        for height in [MIN_PANE, 120.0, 400.0] {
            let metrics = Metrics::pick(Size::new(400.0, height));
            let tallest = metrics.primary_icon() + Metrics::button_padding() * 2.0 + PAD * 2.0;
            assert!(tallest <= height, "height {height} needed {tallest}");
        }
    }

    #[test]
    fn icon_grows_with_pane_height() {
        let short = Metrics::pick(Size::new(400.0, MIN_PANE)).icon;
        let medium = Metrics::pick(Size::new(400.0, 100.0)).icon;
        let tall = Metrics::pick(Size::new(400.0, 300.0)).icon;
        assert!(medium > short, "{medium} should exceed {short}");
        assert!(tall > medium, "{tall} should exceed {medium}");
    }

    #[test]
    fn floor_size_fits_the_smallest_allowed_pane() {
        let needed = ICON_MIN + Metrics::button_padding() * 2.0 + PAD * 2.0;
        assert!(needed <= MIN_PANE, "floor needs {needed}, pane is {MIN_PANE}");
    }

    #[test]
    fn full_row_fits_the_width_it_asks_for() {
        let metrics = Metrics::pick(Size::new(400.0, 80.0));
        let needed = Metrics::full_width(metrics.icon, metrics.spacing);
        assert!(Metrics::pick(Size::new(needed, 80.0)).full);
        assert!(!Metrics::pick(Size::new(needed - 1.0, 80.0)).full);
    }

    #[test]
    fn play_pause_is_the_largest_button() {
        let metrics = Metrics::pick(Size::new(400.0, 80.0));
        assert!(metrics.primary_icon() > metrics.icon);
    }
}
