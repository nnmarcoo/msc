//! A search field: the query and a clear button.
//!
//! Plain composition rather than a custom widget, since nothing here needs to
//! escape its own bounds or handle raw events. It lives in `widgets` anyway
//! because more than one pane draws it, and because the clear button's
//! appear-when-non-empty rule is the sort of thing that drifts when it is
//! rewritten per pane.
//!
//! The bar reports what the user typed and nothing else. The query itself is
//! app state, shared by every pane that lists tracks, so two search panes show
//! the same text and stay in sync; see [`crate::browsing`].
//!
//! The clear button occupies its slot only when there is something to clear.
//! A permanently visible one reads as a control that does nothing most of the
//! time, and reserving the space for it would leave a gap in the common case,
//! so the field simply grows into it.
//!
//! A result count is drawn when the caller supplies one. It matters most when
//! the query hides rows: without it, a filtered list and a short library look
//! identical.

use iced::widget::svg::Handle;
use iced::widget::{button, container, row, svg, text, text_input};
use iced::{Border, Color, Element, Length, Theme};

use crate::styles::{self, LABEL_FONT_SIZE, PAD, radius};

const ICON_CLOSE: &[u8] = include_bytes!("../../../assets/icons/close.svg");

const ICON_SIZE: f32 = 13.0;
const TEXT_SIZE: f32 = 12.0;
const V_PAD: f32 = 5.0;
const PLACEHOLDER: &str = "Search\u{2026}";

pub struct SearchBar<'a, Message> {
    query: &'a str,
    on_input: Box<dyn Fn(String) -> Message + 'a>,
    on_clear: Message,
    count: Option<usize>,
}

impl<'a, Message: Clone + 'a> SearchBar<'a, Message> {
    pub fn new(
        query: &'a str,
        on_input: impl Fn(String) -> Message + 'a,
        on_clear: Message,
    ) -> Self {
        Self {
            query,
            on_input: Box::new(on_input),
            on_clear,
            count: None,
        }
    }

    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    fn is_filtering(&self) -> bool {
        !self.query.trim().is_empty()
    }
}

impl<'a, Message: Clone + 'a> From<SearchBar<'a, Message>> for Element<'a, Message> {
    fn from(bar: SearchBar<'a, Message>) -> Self {
        let filtering = bar.is_filtering();

        let field = text_input(PLACEHOLDER, bar.query)
            .on_input(bar.on_input)
            .size(TEXT_SIZE)
            .padding([V_PAD, PAD])
            .width(Length::Fill)
            .style(field_style);

        let mut line = row![field].spacing(PAD).align_y(iced::Center);

        if let Some(count) = bar.count.filter(|_| filtering) {
            line = line.push(
                text(format!("{count}"))
                    .size(LABEL_FONT_SIZE)
                    .style(dim_style),
            );
        }

        if filtering {
            line = line.push(
                button(icon(ICON_CLOSE))
                    .on_press(bar.on_clear)
                    .padding(2.0)
                    .style(styles::icon_button_style),
            );
        }

        container(line)
            .padding([PAD / 2.0, PAD])
            .width(Length::Fill)
            .into()
    }
}

fn icon<'a, Message: 'a>(bytes: &'static [u8]) -> Element<'a, Message> {
    svg(Handle::from_memory(bytes))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .style(styles::svg_style)
        .into()
}

fn field_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.extended_palette();
    let text_color = palette.background.base.text;

    let border_color = match status {
        text_input::Status::Focused { .. } => palette.primary.base.color,
        _ => palette.background.strong.color,
    };

    text_input::Style {
        background: iced::Background::Color(palette.background.base.color),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius().into(),
        },
        icon: text_color,
        placeholder: Color {
            a: 0.45,
            ..text_color
        },
        value: text_color,
        selection: palette.primary.base.color.scale_alpha(0.35),
    }
}

fn dim_style(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(
            theme
                .extended_palette()
                .background
                .base
                .text
                .scale_alpha(0.55),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(query: &str) -> SearchBar<'_, ()> {
        SearchBar::new(query, |_| (), ())
    }

    #[test]
    fn an_empty_field_has_nothing_to_clear() {
        assert!(!bar("").is_filtering());
    }

    #[test]
    fn whitespace_alone_is_not_a_filter() {
        for query in [" ", "   ", "\t"] {
            assert!(
                !bar(query).is_filtering(),
                "{query:?} counted as a filter, so the list would look filtered \
                 while showing everything"
            );
        }
    }

    #[test]
    fn a_typed_query_can_be_cleared() {
        assert!(bar("blue monday").is_filtering());
    }

    #[test]
    fn a_query_padded_with_spaces_still_filters() {
        assert!(bar("  monday  ").is_filtering());
    }
}
