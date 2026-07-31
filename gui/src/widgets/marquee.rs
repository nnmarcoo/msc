//! Text that ellipsises when it runs out of room, and shows what it hid on hover.
//!
//! A drop-in for [`iced::widget::text`] wherever a label has to share a row with
//! something else: `marquee("a very long title")` lays out and draws like `text`,
//! but a string too wide for the space it is given is cut to fit and finished with
//! an ellipsis rather than wrapping onto a second line.
//!
//! # Why not `text::Wrapping::None`
//!
//! Setting `wrapping` on a [`text::Text`] handed to `Renderer::fill_text` does
//! nothing at all. The renderer stores that text as a `Text::Cached`, a struct
//! with no wrapping field, so the strategy is dropped before it ever reaches
//! cosmic-text and the shaper falls back to its word-wrapping default. Only a
//! `Paragraph` carries wrapping through to the shaper, which is why every line
//! here is shaped into one and drawn with `fill_paragraph`. A widget drawing text
//! by hand and asking for `Wrapping::None` is not choosing anything; it is being
//! quietly ignored.
//!
//! # Why a widget and not a helper
//!
//! The cut depends on the width the widget is given, which nothing knows until
//! [`Widget::layout`] runs. A helper called from view code would have to be told a
//! width its caller has not been told either, so the guess would be wrong exactly
//! when the pane is resized. Doing it in `layout` means the answer is recomputed
//! whenever the space changes, and the shaped result is cached in tree state so
//! that redraws which change nothing cost nothing.
//!
//! The cache is keyed on the text and the width it was cut to, and the widget
//! reshapes only when one of those actually moves. That is load-bearing rather
//! than an optimization: shaping is the expensive half of this widget, and
//! `layout` runs on every frame a pane is resized.
//!
//! [`width_of`] exposes the same measurement to callers who have to decide what to
//! *put* in a row before building it, so a pane can drop one thing to keep another
//! whole. Comparing real widths is the only way to make that call; a guess at how
//! wide a title tends to be is wrong in both directions at once.
//!
//! # Why the ellipsis is a search
//!
//! A proportional font gives no relation between a prefix's length and its width,
//! so the longest prefix that fits cannot be computed: it has to be found, by
//! bisecting the char boundaries and shaping each candidate. That is why the
//! result is cached rather than recomputed per frame.
//!
//! Cuts land on `char` boundaries. That splits a grapheme cluster in the worst
//! case (a family emoji, a combining accent) but never a UTF-8 sequence, and the
//! shaper re-clusters whatever it is handed. A segmentation crate would buy
//! correctness only for strings that are already being cut.
//!
//! # The tooltip
//!
//! Only a line that is actually ellipsised gets one: a label showing its full text
//! has nothing to reveal, and a tooltip there would be noise. It is a true
//! [`Overlay`] so it can escape the clip bounds of a narrow pane, and it claims the
//! cursor over every pixel it covers for the reason `docs/overlay-cursor.md` gives.
//!
//! Hover comes from the `cursor` argument, never from the position carried on the
//! event. The two differ exactly when something is open above: `user_interface`
//! hands the base layer `Cursor::Unavailable` once an overlay claims the pointer,
//! while the `CursorMoved` event is still delivered underneath. Reading the event
//! would therefore raise a tooltip out from under an open context menu, which is
//! the same fall-through `docs/overlay-cursor.md` describes, arriving on the input
//! side instead of the drawing side.

use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::{self, Paragraph as _};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay};
use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Pixels, Point, Rectangle, Renderer,
    Size, Theme, Vector,
};

use crate::styles::radius;

type Plain = text::paragraph::Plain<<Renderer as text::Renderer>::Paragraph>;

type Styler<'a> = Box<dyn Fn(&Theme) -> Color + 'a>;

pub fn width_of(content: &str, size: f32, font: iced::Font) -> f32 {
    Format { size, font }.measure(content)
}

const ELLIPSIS: &str = "\u{2026}";

const TIP_PAD: Padding = Padding {
    top: 4.0,
    right: 8.0,
    bottom: 4.0,
    left: 8.0,
};

const TIP_OFFSET: f32 = 20.0;

pub fn marquee<'a>(content: impl text::IntoFragment<'a>) -> Marquee<'a> {
    Marquee::new(content)
}

pub struct Marquee<'a> {
    content: text::Fragment<'a>,
    size: Option<f32>,
    font: Option<iced::Font>,
    style: Option<Styler<'a>>,
    width: Length,
    tooltip: bool,
}

impl<'a> Marquee<'a> {
    pub fn new(content: impl text::IntoFragment<'a>) -> Self {
        Self {
            content: content.into_fragment(),
            size: None,
            font: None,
            style: None,
            width: Length::Shrink,
            tooltip: true,
        }
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into().0);
        self
    }

    pub fn font(mut self, font: iced::Font) -> Self {
        self.font = Some(font);
        self
    }

    pub fn style(mut self, style: impl Fn(&Theme) -> Color + 'a) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn tooltip(mut self, tooltip: bool) -> Self {
        self.tooltip = tooltip;
        self
    }

    fn format(&self, renderer: &Renderer) -> Format {
        use iced::advanced::text::Renderer as _;

        Format {
            size: self.size.unwrap_or_else(|| renderer.default_size().0),
            font: self.font.unwrap_or_else(|| renderer.default_font()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Format {
    size: f32,
    font: iced::Font,
}

impl Default for Format {
    fn default() -> Self {
        Self {
            size: 0.0,
            font: iced::Font::DEFAULT,
        }
    }
}

impl Format {
    fn line_height(self) -> f32 {
        text::LineHeight::default().to_absolute(Pixels(self.size)).0
    }

    fn shaped(self, content: &str) -> text::Text<&str> {
        text::Text {
            content,
            bounds: Size::new(f32::INFINITY, self.line_height()),
            size: Pixels(self.size),
            line_height: text::LineHeight::default(),
            font: self.font,
            align_x: text::Alignment::Left,
            align_y: iced::alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
        }
    }

    fn measure(self, content: &str) -> f32 {
        <Renderer as text::Renderer>::Paragraph::with_text(self.shaped(content))
            .min_bounds()
            .width
    }
}

#[derive(Default)]
struct State {
    shown: Plain,
    full: String,
    format: Format,
    natural: f32,
    cut_to: f32,
    cut: bool,
    hovered: bool,
    tip: Plain,
}

impl<Message> Widget<Message, Theme, Renderer> for Marquee<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let format = self.format(renderer);
        let state = tree.state.downcast_mut::<State>();

        layout::sized(limits, self.width, Length::Shrink, |limits| {
            let room = limits.max().width;
            let restated = state.full.as_str() != self.content.as_ref() || state.format != format;

            if restated {
                state.full.clear();
                state.full.push_str(&self.content);
                state.format = format;
                state.natural = format.measure(&self.content);
            }

            let cut = state.natural > room;
            if restated || cut != state.cut || (cut && (state.cut_to - room).abs() >= f32::EPSILON)
            {
                state.cut = cut;
                state.cut_to = room;

                if cut {
                    let _ = state.shown.update(format.shaped(&longest_prefix(
                        &self.content,
                        room,
                        format,
                    )));
                    let _ = state.tip.update(format.shaped(&self.content));
                } else {
                    let _ = state.shown.update(format.shaped(&self.content));
                }
            }

            Size::new(
                state.shown.min_bounds().width.min(room),
                format.line_height(),
            )
        })
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
        if !self.tooltip
            || !matches!(
                event,
                Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft)
            )
        {
            return;
        }

        let state = tree.state.downcast_mut::<State>();
        let hovered = state.cut && cursor.is_over(layout.bounds());
        if hovered != state.hovered {
            state.hovered = hovered;
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::text::Renderer as _;

        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        let color = self
            .style
            .as_ref()
            .map_or(style.text_color, |style| style(theme));

        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };

        renderer.fill_paragraph(state.shown.raw(), bounds.position(), color, clip);
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        offset: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State>();
        if !self.tooltip || !state.cut || !state.hovered {
            return None;
        }

        Some(overlay::Element::new(Box::new(Tip {
            text: &state.tip,
            anchor: layout.bounds().position() + offset,
            height: layout.bounds().height,
        })))
    }
}

struct Tip<'b> {
    text: &'b Plain,
    anchor: Point,
    height: f32,
}

impl<Message> overlay::Overlay<Message, Theme, Renderer> for Tip<'_> {
    fn layout(&mut self, _renderer: &Renderer, bounds: Size) -> layout::Node {
        let text = self.text.min_bounds();
        let size = Size::new(
            text.width + TIP_PAD.left + TIP_PAD.right,
            text.height + TIP_PAD.top + TIP_PAD.bottom,
        );

        let below = self.anchor.y + self.height + TIP_OFFSET;
        let y = if below + size.height > bounds.height {
            (self.anchor.y - size.height - 4.0).max(0.0)
        } else {
            below
        };
        let x = self.anchor.x.min((bounds.width - size.width).max(0.0));

        layout::Node::new(size).move_to(Point::new(x, y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
    ) {
        use iced::advanced::Renderer as _;
        use iced::advanced::text::Renderer as _;

        let bounds = layout.bounds();
        let palette = theme.extended_palette();

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    radius: radius().into(),
                    width: 1.0,
                    color: palette.background.strong.color,
                },
                ..Quad::default()
            },
            Background::Color(palette.background.weak.color),
        );

        renderer.fill_paragraph(
            self.text.raw(),
            bounds.position() + Vector::new(TIP_PAD.left, TIP_PAD.top),
            palette.background.base.text,
            bounds,
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Idle
        } else {
            mouse::Interaction::None
        }
    }
}

impl<'a, Message: 'a> From<Marquee<'a>> for Element<'a, Message, Theme, Renderer> {
    fn from(marquee: Marquee<'a>) -> Self {
        Self::new(marquee)
    }
}

fn longest_prefix(content: &str, width: f32, format: Format) -> String {
    let cuts: Vec<usize> = content
        .char_indices()
        .map(|(at, _)| at)
        .chain(std::iter::once(content.len()))
        .collect();

    let fits = |end: usize| {
        let mut candidate = String::with_capacity(end + ELLIPSIS.len());
        candidate.push_str(&content[..end]);
        candidate.push_str(ELLIPSIS);
        format.measure(&candidate) <= width
    };

    let (mut lo, mut hi) = (0, cuts.len() - 1);
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if fits(cuts[mid]) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    let mut out = String::from(&content[..cuts[lo]]);
    out.push_str(ELLIPSIS);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format() -> Format {
        Format {
            size: 13.0,
            font: iced::Font::DEFAULT,
        }
    }

    #[test]
    fn a_declared_width_reaches_the_layout() {
        let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 100.0));
        let node = layout::sized(&limits, Length::Fill, Length::Shrink, |limits| {
            Size::new(format().measure("Hi").min(limits.max().width), 17.0)
        });

        assert!(
            (node.size().width - 400.0).abs() < 0.001,
            "a Fill width laid out at {} rather than filling its 400px row",
            node.size().width
        );
    }

    #[test]
    fn a_shrunk_width_takes_only_what_the_text_needs() {
        let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 100.0));
        let wanted = format().measure("Hi");
        let node = layout::sized(&limits, Length::Shrink, Length::Shrink, |limits| {
            Size::new(wanted.min(limits.max().width), 17.0)
        });

        assert!(
            (node.size().width - wanted).abs() < 0.001,
            "a Shrink width claimed {} of a 400px row",
            node.size().width
        );
    }

    #[test]
    fn a_cut_line_ends_in_an_ellipsis() {
        let cut = longest_prefix("Black Country, New Road", 60.0, format());
        assert!(cut.ends_with(ELLIPSIS), "{cut:?} lost its ellipsis");
    }

    #[test]
    fn a_cut_line_fits_the_width_it_was_cut_to() {
        let format = format();
        for width in [20.0, 40.0, 80.0, 160.0] {
            let cut = longest_prefix("Current Through a Blood Rushing Like Nectar", width, format);
            assert!(
                format.measure(&cut) <= width,
                "{cut:?} is {} wide, over {width}",
                format.measure(&cut)
            );
        }
    }

    #[test]
    fn a_cut_line_is_the_longest_that_fits() {
        let format = format();
        let content = "Current Through a Blood Rushing Like Nectar";
        let width = 80.0;

        let cut = longest_prefix(content, width, format);
        let kept = cut.strip_suffix(ELLIPSIS).expect("no ellipsis");

        let next = content[kept.len()..].chars().next();
        if let Some(next) = next {
            let mut longer = String::from(kept);
            longer.push(next);
            longer.push_str(ELLIPSIS);
            assert!(
                format.measure(&longer) > width,
                "{longer:?} also fits, so the cut was too eager"
            );
        }
    }

    #[test]
    fn a_box_too_narrow_for_anything_still_draws_the_ellipsis() {
        let cut = longest_prefix("Black Country, New Road", 0.0, format());
        assert_eq!(cut, ELLIPSIS);
    }

    #[test]
    fn a_cut_never_splits_a_utf8_sequence() {
        let format = format();
        for content in ["Пётр Ильич Чайковский", "夜に駆ける", "Björk — Jóga"]
        {
            for width in [10.0, 25.0, 50.0, 90.0] {
                let cut = longest_prefix(content, width, format);
                assert!(cut.ends_with(ELLIPSIS));
            }
        }
    }

    #[test]
    fn an_empty_line_is_just_the_ellipsis() {
        assert_eq!(longest_prefix("", 100.0, format()), ELLIPSIS);
    }
}
