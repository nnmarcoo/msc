//! A row of stars that reads and sets a track's rating.
//!
//! A custom widget rather than a row of buttons because rating is one gesture
//! across five targets, not five independent ones: the row previews the value
//! under the pointer, so hovering the third star must light the first two as
//! well. Buttons would each have to be told what their neighbors were doing.
//! Handling the row as a unit also makes the hit target a simple division of
//! the row's width, which is what lets the stars stay small without becoming
//! hard to aim at.
//!
//! The widget never keeps its own copy of the rating. It draws what it is
//! handed and reports what was clicked, so a rating set here and one set from
//! a context menu or a track list cannot drift apart.
//!
//! Clicking the star already set clears the rating. Once a track has stars
//! there is otherwise no way back to unrated, since every star in the row means
//! "at least this many".
//!
//! `on_rate` is optional, and its absence is what makes the row read-only: a
//! list showing ratings it does not want edited passes `None` and gets the same
//! glyphs without hover, pointer cursor, or clicks. That is one widget for both
//! uses rather than a separate display-only one that would drift in appearance.
//!
//! Sizing is the caller's, since the row is drawn at label scale inside the
//! timeline and could be drawn larger in an inspector. The row asks for exactly
//! the width its stars need so it can be laid out beside text without a
//! container measuring it.
//!
//! A lit star is drawn in an accent, which the caller may tint by the playing
//! record — see [`crate::pane::settings::Accent`]. Hovering changes only *how
//! many* stars are lit, never what color they are: the preview and the rating it
//! would replace are the same reading at two values, so coloring them
//! differently made the row appear to change meaning under the pointer rather
//! than to be counting. An unlit star keeps the text color at low alpha, since
//! it is the empty slot the count is read against rather than part of the count.
//!
//! That the lit color is the accent rather than the palette's text is what puts
//! the row in step with the timeline it sits in: the stars, the rail below them
//! and the title beside them all take the same color, so a pane following the
//! artwork tints as one thing.

use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::widget::svg::Handle;
use iced::{Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, mouse};

use crate::pane::settings::Accent;

use std::sync::LazyLock;

const ICON_STAR: &[u8] = include_bytes!("../../../assets/icons/star.svg");
const ICON_STAR_OUTLINE: &[u8] = include_bytes!("../../../assets/icons/star_outline.svg");

static STAR: LazyLock<Handle> = LazyLock::new(|| Handle::from_memory(ICON_STAR));
static STAR_OUTLINE: LazyLock<Handle> = LazyLock::new(|| Handle::from_memory(ICON_STAR_OUTLINE));

pub const STARS: u8 = 5;

const DEFAULT_SIZE: f32 = 13.0;
const DEFAULT_SPACING: f32 = 2.0;

#[derive(Default)]
struct State {
    hovered: Option<u8>,
}

pub struct Rating<'a, Message> {
    stars: Option<u8>,
    size: f32,
    spacing: f32,
    accent: Accent,
    cover: Option<[u8; 3]>,
    on_rate: Option<Box<dyn Fn(Option<u8>) -> Message + 'a>>,
}

impl<'a, Message> Rating<'a, Message> {
    pub fn new(stars: Option<u8>) -> Self {
        Self {
            stars,
            size: DEFAULT_SIZE,
            spacing: DEFAULT_SPACING,
            accent: Accent::default(),
            cover: None,
            on_rate: None,
        }
    }

    pub fn accent(mut self, accent: Accent, cover: Option<[u8; 3]>) -> Self {
        self.accent = accent;
        self.cover = cover;
        self
    }

    pub fn on_rate(mut self, on_rate: impl Fn(Option<u8>) -> Message + 'a) -> Self {
        self.on_rate = Some(Box::new(on_rate));
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn width_for(size: f32, spacing: f32) -> f32 {
        f32::from(STARS) * size + f32::from(STARS - 1) * spacing
    }

    fn slot(&self) -> f32 {
        self.size + self.spacing
    }

    fn row_width(&self) -> f32 {
        Self::width_for(self.size, self.spacing)
    }

    fn is_editable(&self) -> bool {
        self.on_rate.is_some()
    }

    fn star_at(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<u8> {
        let position = cursor.position()?;
        if !bounds.contains(position) {
            return None;
        }
        let index = ((position.x - bounds.x) / self.slot()).floor();
        let stars = index.max(0.0) as u8 + 1;
        (1..=STARS).contains(&stars).then_some(stars)
    }

    fn clicked(&self, stars: u8) -> Option<u8> {
        (self.stars != Some(stars)).then_some(stars)
    }

    fn lit(&self, palette: &iced::theme::palette::Extended) -> Color {
        self.accent.resolve(palette.primary.base.color, self.cover)
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Rating<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(self.row_width()),
            height: Length::Fixed(self.size),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(self.row_width(), self.size))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();

        let Some(on_rate) = self.on_rate.as_ref() else {
            state.hovered = None;
            return;
        };

        let bounds = layout.bounds();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(stars) = self.star_at(bounds, cursor) {
                    shell.publish(on_rate(self.clicked(stars)));
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let over = self.star_at(bounds, cursor);
                if over != state.hovered {
                    state.hovered = over;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorLeft) if state.hovered.take().is_some() => {
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let palette = theme.extended_palette();
        let state = tree.state.downcast_ref::<State>();

        let shown = state.hovered.or(self.stars).unwrap_or(0);
        let lit = self.lit(palette);
        let unlit = palette.background.base.text.scale_alpha(0.35);

        for index in 0..STARS {
            let filled = index < shown;
            draw_star(
                renderer,
                Rectangle {
                    x: bounds.x + f32::from(index) * self.slot(),
                    y: bounds.y,
                    width: self.size,
                    height: self.size,
                },
                if filled { lit } else { unlit },
                filled,
                *viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.is_editable() && self.star_at(layout.bounds(), cursor).is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn draw_star(
    renderer: &mut Renderer,
    bounds: Rectangle,
    color: Color,
    filled: bool,
    viewport: Rectangle,
) {
    use iced::advanced::svg::Renderer as _;

    let handle = if filled { &*STAR } else { &*STAR_OUTLINE };

    renderer.draw_svg(
        iced::advanced::svg::Svg {
            handle: handle.clone(),
            color: Some(color),
            rotation: iced::Radians(0.0),
            opacity: 1.0,
        },
        bounds,
        viewport,
    );
}

impl<'a, Message: 'a> From<Rating<'a, Message>> for Element<'a, Message, Theme, Renderer> {
    fn from(rating: Rating<'a, Message>) -> Self {
        Self::new(rating)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;

    fn row(rating: Option<u8>) -> Rating<'static, ()> {
        Rating::new(rating).on_rate(|_| ())
    }

    fn bounds(row: &Rating<'_, ()>) -> Rectangle {
        Rectangle {
            x: 10.0,
            y: 20.0,
            width: row.row_width(),
            height: row.size,
        }
    }

    fn at(bounds: Rectangle, x: f32) -> mouse::Cursor {
        mouse::Cursor::Available(Point::new(x, bounds.center_y()))
    }

    #[test]
    fn each_star_claims_its_own_slot() {
        let row = row(None);
        let area = bounds(&row);

        let hit: Vec<Option<u8>> = (0..STARS)
            .map(|index| {
                let x = area.x + f32::from(index) * row.slot() + row.size / 2.0;
                row.star_at(area, at(area, x))
            })
            .collect();

        assert_eq!(hit, vec![Some(1), Some(2), Some(3), Some(4), Some(5)]);
    }

    #[test]
    fn pointing_outside_the_row_rates_nothing() {
        let row = row(None);
        let area = bounds(&row);

        assert_eq!(row.star_at(area, at(area, area.x - 1.0)), None);
        assert_eq!(
            row.star_at(area, at(area, area.x + area.width + 1.0)),
            None,
            "past the last star should not rate"
        );
    }

    #[test]
    fn a_cursor_above_the_row_rates_nothing() {
        let row = row(None);
        let area = bounds(&row);
        let above = mouse::Cursor::Available(Point::new(area.center_x(), area.y - 5.0));

        assert_eq!(row.star_at(area, above), None);
    }

    #[test]
    fn clicking_the_star_already_set_clears_the_rating() {
        assert_eq!(
            row(Some(3)).clicked(3),
            None,
            "clicking the third star of a three-star track should unrate it"
        );
    }

    #[test]
    fn clicking_a_different_star_sets_it() {
        let row = row(Some(3));
        for stars in [1, 2, 4, 5] {
            assert_eq!(row.clicked(stars), Some(stars));
        }
    }

    #[test]
    fn an_unrated_track_takes_the_star_that_was_clicked() {
        assert_eq!(row(None).clicked(4), Some(4));
    }

    #[test]
    fn a_row_without_a_handler_is_read_only() {
        let display: Rating<'_, ()> = Rating::new(Some(3));
        assert!(!display.is_editable());
        assert!(row(Some(3)).is_editable());
    }

    #[test]
    fn the_row_is_as_wide_as_its_stars_and_gaps() {
        let row = row(None).size(10.0).spacing(4.0);
        assert!((row.row_width() - (5.0 * 10.0 + 4.0 * 4.0)).abs() < 0.001);
    }

    #[test]
    fn the_advertised_width_matches_what_is_laid_out() {
        let row = row(None).size(20.0).spacing(3.0);
        assert!((Rating::<()>::width_for(20.0, 3.0) - row.row_width()).abs() < 0.001);
    }

    const RED_SLEEVE: [u8; 3] = [150, 40, 40];

    fn lit(row: &Rating<'_, ()>, theme: &Theme) -> Color {
        row.lit(theme.extended_palette())
    }

    fn primary(theme: &Theme) -> Color {
        theme.extended_palette().primary.base.color
    }

    #[test]
    fn a_row_following_the_theme_lights_in_the_theme_s_primary() {
        let theme = Theme::Dark;
        assert_eq!(lit(&row(Some(2)), &theme), primary(&theme));
    }

    #[test]
    fn a_row_following_the_artwork_lights_in_the_record_s_color() {
        let theme = Theme::Dark;
        let row = row(Some(2)).accent(Accent::Artwork, Some(RED_SLEEVE));
        let tinted = lit(&row, &theme);

        assert!(
            tinted.r > tinted.b,
            "a red sleeve did not reach the stars: {tinted:?}"
        );
        assert_ne!(tinted, primary(&theme));
    }

    #[test]
    fn the_stars_light_in_the_same_color_the_rail_fills_with() {
        for theme in crate::config::ALL_THEMES {
            let row = row(Some(2)).accent(Accent::Artwork, Some(RED_SLEEVE));
            let rail = Accent::Artwork.resolve(primary(theme), Some(RED_SLEEVE));

            assert_eq!(
                lit(&row, theme),
                rail,
                "{theme} lights its stars and its rail differently"
            );
        }
    }

    #[test]
    fn a_track_with_no_cover_lights_in_the_theme_s_primary() {
        let theme = Theme::Dark;
        let row = row(Some(2)).accent(Accent::Artwork, None);

        assert_eq!(
            lit(&row, &theme),
            primary(&theme),
            "a row with no cover to read should fall back rather than draw nothing"
        );
    }

    #[test]
    fn a_lit_star_is_never_the_plain_text_color() {
        for accent in [Accent::Theme, Accent::Artwork] {
            for theme in crate::config::ALL_THEMES {
                let row = row(Some(2)).accent(accent, Some(RED_SLEEVE));
                let palette = theme.extended_palette();

                assert_ne!(
                    row.lit(palette),
                    palette.background.base.text,
                    "{theme} lights its stars in the plain text color, so a pane \
                     following the artwork tints everything around them but not them"
                );
            }
        }
    }

    #[test]
    fn a_read_only_row_lights_the_same_as_an_editable_one() {
        let theme = Theme::Dark;
        let display: Rating<'_, ()> =
            Rating::new(Some(3)).accent(Accent::Artwork, Some(RED_SLEEVE));
        let editable = row(Some(3)).accent(Accent::Artwork, Some(RED_SLEEVE));

        assert_eq!(
            display.lit(theme.extended_palette()),
            editable.lit(theme.extended_palette()),
            "a rating shown in a list drew a different color than the same rating \
             shown where it can be clicked"
        );
    }
}
