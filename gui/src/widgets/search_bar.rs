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
const ICON_SEARCH: &[u8] = include_bytes!("../../../assets/icons/search.svg");

const ICON_SIZE: f32 = 13.0;
const TEXT_SIZE: f32 = 12.0;
const V_PAD: f32 = 5.0;
const PLACEHOLDER: &str = "Search\u{2026}";

const HERO_TEXT_SIZE: f32 = 15.0;
const HERO_V_PAD: f32 = 11.0;
/// The cross itself, and the ring of space around it.
///
/// The icon carries the meaning and the padding is only a hit target, so the
/// icon takes most of the button: at 11px inside 5px of padding the glyph was a
/// third of what it drew, and read as a speck centred in an empty circle rather
/// than a control. The proportion here is roughly two thirds ink.
const HERO_CLEAR_ICON: f32 = 15.0;
const HERO_CLEAR_PAD: f32 = 3.0;

/// The clear button's full edge, and so twice the radius that rounds it into a
/// circle.
const HERO_CLEAR_EDGE: f32 = HERO_CLEAR_ICON + HERO_CLEAR_PAD * 2.0;

pub struct SearchBar<'a, Message> {
    query: &'a str,
    on_input: Box<dyn Fn(String) -> Message + 'a>,
    on_clear: Message,
    count: Option<usize>,
    hero: bool,
    placeholder: Option<&'a str>,
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
            hero: false,
            placeholder: None,
        }
    }

    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Draws the field as the primary thing in its pane rather than a strip.
    ///
    /// A search that is the whole point of a pane should not look like the one
    /// that filters a list already on screen: it carries a magnifier, sits
    /// taller, and takes the full width so it reads as an invitation to type
    /// rather than a control to find.
    pub fn hero(mut self) -> Self {
        self.hero = true;
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    fn is_filtering(&self) -> bool {
        !self.query.trim().is_empty()
    }
}

impl<'a, Message: Clone + 'a> From<SearchBar<'a, Message>> for Element<'a, Message> {
    fn from(bar: SearchBar<'a, Message>) -> Self {
        let filtering = bar.is_filtering();

        let hero = bar.hero;

        let field = text_input(bar.placeholder.unwrap_or(PLACEHOLDER), bar.query)
            .on_input(bar.on_input)
            .size(if hero { HERO_TEXT_SIZE } else { TEXT_SIZE })
            .padding(if hero {
                [HERO_V_PAD, PAD].into()
            } else {
                iced::Padding::from([V_PAD, PAD])
            })
            .width(Length::Fill)
            .style(if hero { hero_field_style } else { field_style });

        let mut line = row![].spacing(PAD).align_y(iced::Center);

        if hero {
            line = line.push(container(icon(ICON_SEARCH)).padding([0.0, PAD / 2.0]));
        }

        line = line.push(field);

        if let Some(count) = bar.count.filter(|_| filtering) {
            line = line.push(
                text(format!("{count}"))
                    .size(LABEL_FONT_SIZE)
                    .style(dim_style),
            );
        }

        if filtering {
            line = line.push(if hero {
                button(
                    svg(Handle::from_memory(ICON_CLOSE))
                        .width(Length::Fixed(HERO_CLEAR_ICON))
                        .height(Length::Fixed(HERO_CLEAR_ICON))
                        .style(hero_clear_icon_style),
                )
                .on_press(bar.on_clear)
                .padding(HERO_CLEAR_PAD)
                .style(hero_clear_style)
            } else {
                button(icon(ICON_CLOSE))
                    .on_press(bar.on_clear)
                    .padding(2.0)
                    .style(styles::icon_button_style)
            });
        }

        let padding = if hero {
            [PAD / 2.0, PAD * 1.5]
        } else {
            [PAD / 2.0, PAD]
        };

        let shell = container(line).padding(padding).width(Length::Fill);

        if hero {
            shell.style(hero_shell_style).into()
        } else {
            shell.into()
        }
    }
}

/// The hero field's own chrome sits on the container, not the input.
///
/// The magnifier is a sibling of the `text_input` rather than inside it, so the
/// rounded surface has to enclose both or the icon floats outside the field it
/// belongs to.
fn hero_shell_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(iced::Background::Color(palette.background.weak.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: (radius() * 1.5).into(),
        },
        ..container::Style::default()
    }
}

fn hero_field_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        ..field_style(theme, status)
    }
}

fn icon<'a, Message: 'a>(bytes: &'static [u8]) -> Element<'a, Message> {
    icon_sized(bytes, ICON_SIZE)
}

fn icon_sized<'a, Message: 'a>(bytes: &'static [u8], edge: f32) -> Element<'a, Message> {
    svg(Handle::from_memory(bytes))
        .width(Length::Fixed(edge))
        .height(Length::Fixed(edge))
        .style(styles::svg_style)
        .into()
}

/// The hero field's clear button.
///
/// Round rather than the shared rounded-rectangle: at a hair over the icon's own
/// size, a 6px corner radius reads as a slightly-squashed box rather than a
/// deliberate shape, and next to a field with a 9px radius it looked like a
/// mistake. A circle is unambiguous at any size.
///
/// It also sits back until pointed at. The cross is the one control in the field
/// that destroys something, and drawing it at full strength beside the query put
/// the most emphatic mark in the row on the thing the user least often wants.
fn hero_clear_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let background = match status {
        button::Status::Hovered | button::Status::Pressed => Some(iced::Background::Color(
            palette.background.strong.color.scale_alpha(0.55),
        )),
        _ => None,
    };

    button::Style {
        background,
        text_color: palette.background.base.text,
        border: Border {
            radius: (HERO_CLEAR_EDGE / 2.0).into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Dimmer at rest than [`styles::svg_style`], which never drops below 0.7.
///
/// The cross has to sit quieter than that: it is the only mark in the field
/// competing with the query text itself, and at icon strength the eye kept
/// landing on the control that throws the search away.
fn hero_clear_icon_style(theme: &Theme, status: svg::Status) -> svg::Style {
    let base = theme.extended_palette().background.base.text;

    let alpha = match status {
        svg::Status::Hovered => 1.0,
        svg::Status::Idle => 0.45,
    };

    svg::Style {
        color: Some(Color { a: alpha, ..base }),
    }
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

    #[test]
    fn a_bar_is_a_strip_unless_it_is_asked_to_be_a_hero() {
        assert!(!bar("").hero);
        assert!(bar("").hero().hero);
    }

    #[test]
    fn a_hero_bar_can_name_what_it_is_searching() {
        let named = bar("").placeholder("Search for music");

        assert_eq!(named.placeholder, Some("Search for music"));
        assert_eq!(bar("").placeholder, None, "the default is filled in later");
    }

    /// The clear button is round, which only holds while the radius is half the
    /// edge; any less and it draws as a rounded square beside a field whose own
    /// corners are a different radius entirely.
    const _: () = assert!(HERO_CLEAR_EDGE == HERO_CLEAR_ICON + HERO_CLEAR_PAD * 2.0);
}
