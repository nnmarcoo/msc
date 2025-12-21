use iced::widget::slider::{self, Handle, HandleShape, Rail, Slider, Status};
use iced::{Border, Color, Theme};

pub fn hover_slider<'a, Message: Clone + 'a>(
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Slider<'a, f32, Message> {
    Slider::new(range, value, on_change).style(style)
}

fn style(theme: &Theme, status: Status) -> slider::Style {
    let palette = theme.extended_palette();

    let handle = match status {
        Status::Hovered | Status::Dragged => Handle {
            shape: HandleShape::Circle { radius: 6.0 },
            background: palette.primary.strong.color.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
        Status::Active => Handle {
            shape: HandleShape::Circle { radius: 2.0 },
            background: palette.primary.strong.color.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    };

    slider::Style {
        rail: Rail {
            backgrounds: (
                palette.primary.strong.color.into(),
                palette.background.strong.color.into(),
            ),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
        },
        handle,
    }
}
