//! A search-first pick list for choosing what a pane contains.
//!
//! The trigger is a small labeled button that lives in the pane's edit
//! overlay; pressing it opens a panel as a true iced `Overlay`, so the list
//! escapes the pane's clip bounds and can be larger than the pane it belongs
//! to. That is the whole reason this is a custom widget rather than a
//! `stack!` layer, so a pane can be 80px tall and still open a full list.
//!
//! The panel is rebuilt only when the query changes ([`State::built_for`]),
//! because building it allocates an `Element` tree that then has to be
//! diffed into a `Tree`. Item presses travel back as [`Op`]s through a local
//! `Shell` and are translated into the caller's message, so the widget stays
//! generic over `Message`.
//!
//! The trigger has two widths, since the edit overlay measures it before laying
//! out: [`PanePicker::label_width`] with the pane's title, and
//! [`PanePicker::compact_width`] once it drops the label for its icon.
//! `text_size` scales both the width and the height, so a caller making the
//! picker a heading gets a trigger that grew to fit rather than a large label in
//! a small box.
//!
//! That width is estimated per character rather than measured, in three glyph
//! classes. One average ratio has to be wrong for something: at 0.58 it sized
//! every title as if it were all capitals and left the surplus as slack around
//! short ones, and at 0.5 it fitted lowercase exactly and clipped the leading
//! capital every title has. The trigger carries no padding to hide an
//! underestimate in and the label is centered, so a budget short of the text
//! clips both ends of it — hence the classes, plus slack on top for the rounding
//! an approximation accumulates.
//!
//! That slack has two parts because the error has two parts. `GLYPH_SLACK` is a
//! fixed allowance, and `GLYPH_SLACK_RATIO` scales with the estimate, because
//! per-glyph error accumulates with length: a fixed allowance alone is generous
//! for "Queue" and short for "Track Information", which is exactly where it
//! first failed to cover. The ratio is `CAP_RATIO / LOWER_RATIO - 1`, the gap
//! between estimating a glyph as lowercase and the worst case of it being a
//! capital, so the budget holds for a title of any length rather than only the
//! ones that exist today, and the fixed part is margin rather than the thing
//! load-bearing for long titles.
//!
//! `PickerOverlay::mouse_interaction` claims the cursor over the whole panel,
//! not just its interactive parts: iced hands the cursor back to the layer
//! beneath whenever an overlay answers `Interaction::None`, so the gaps between
//! items would let the pane behind this one hover through the panel. That covers
//! the panel alone; while a category fly-out is open, `overlay::Nested` asks only
//! the fly-out, which answers for this panel in turn. See
//! `docs/overlay-cursor.md`.

use std::cell::Cell;
use std::rc::Rc;

use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::Renderer as _;
use iced::advanced::widget::operation::focusable;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Overlay, Shell, Widget, layout, text};
use iced::alignment::Vertical;
use iced::keyboard::key::Named;
use iced::keyboard::{self, Key};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, svg, text as text_widget, text_input};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme,
    Vector, mouse, overlay,
};

use crate::pane::PaneKind;
use crate::styles::{LABEL_FONT_SIZE, radius};
use crate::widgets::menu::{SubMenuSide, menu_item, styled_menu, sub_menu};

const TRIGGER_HEIGHT: f32 = 20.0;
const TRIGGER_PADDING_V: f32 = 3.0;

const CAP_RATIO: f32 = 0.62;
const LOWER_RATIO: f32 = 0.5;
const SPACE_RATIO: f32 = 0.28;

const GLYPH_SLACK: f32 = 1.4;
const GLYPH_SLACK_RATIO: f32 = CAP_RATIO / LOWER_RATIO - 1.0;
const COMPACT_TRIGGER_WIDTH: f32 = 24.0;
const COMPACT_ICON_SIZE: f32 = 14.0;

const ICON_PANE: &[u8] = include_bytes!("../../../assets/icons/pane.svg");

const PANEL_WIDTH: f32 = 160.0;
const SUBMENU_WIDTH: f32 = 168.0;
const PADDING: f32 = 6.0;
const GAP: f32 = 4.0;

const SEARCH_V_PAD: f32 = 6.0;
const SEARCH_TEXT_SIZE: f32 = 12.0;
const SEARCH_HEIGHT: f32 = SEARCH_TEXT_SIZE + 2.0 * SEARCH_V_PAD + 2.0;
const SEARCH_PLACEHOLDER: &str = "Search panes\u{2026}";
const SEARCH_ID: &str = "pane_picker_search";

const ITEM_HEIGHT: f32 = 24.0;
const ITEM_SPACING: f32 = 2.0;
const ITEM_PADDING_H: f32 = 8.0;
const ITEM_TEXT_SIZE: f32 = 12.0;

const HEADER_TEXT_SIZE: f32 = 10.0;

const MAX_LIST_HEIGHT: f32 = 320.0;
const SCROLLBAR_WIDTH: f32 = 4.0;
const SCROLLBAR_GUTTER: f32 = 4.0;

#[derive(Clone)]
enum Op {
    Query(String),
    Pick(PaneKind),
}

#[derive(Default)]
struct State {
    open: bool,
    needs_focus: bool,
    query: String,
    built_for: Option<String>,
    content: Option<Element<'static, Op, Theme, Renderer>>,
    panel_tree: Option<Tree>,
    picked: Rc<Cell<bool>>,
}

pub struct PanePicker<Message> {
    current: PaneKind,
    on_select: Box<dyn Fn(PaneKind) -> Message>,
    compact: bool,
    text_size: f32,
}

impl<Message> PanePicker<Message> {
    pub fn new(current: PaneKind, on_select: impl Fn(PaneKind) -> Message + 'static) -> Self {
        Self {
            current,
            on_select: Box::new(on_select),
            compact: false,
            text_size: LABEL_FONT_SIZE,
        }
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = size;
        self
    }

    pub fn label_width(kind: PaneKind) -> f32 {
        Self::label_width_at(kind, LABEL_FONT_SIZE)
    }

    fn label_width_at(kind: PaneKind, text_size: f32) -> f32 {
        let advances: f32 = kind
            .title()
            .chars()
            .map(|glyph| {
                if glyph == ' ' {
                    SPACE_RATIO
                } else if glyph.is_uppercase() {
                    CAP_RATIO
                } else {
                    LOWER_RATIO
                }
            })
            .sum();

        (advances * (1.0 + GLYPH_SLACK_RATIO) + GLYPH_SLACK) * text_size
    }

    fn trigger_height(&self) -> f32 {
        TRIGGER_HEIGHT.max(self.text_size + TRIGGER_PADDING_V * 2.0)
    }

    pub fn compact_width() -> f32 {
        COMPACT_TRIGGER_WIDTH
    }

    fn trigger_width(&self) -> f32 {
        if self.compact {
            Self::compact_width()
        } else {
            Self::label_width_at(self.current, self.text_size)
        }
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for PanePicker<Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Shrink,
            height: Length::Fixed(self.trigger_height()),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(
            limits,
            Length::Fixed(self.trigger_width()),
            Length::Fixed(self.trigger_height()),
        )
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
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && cursor.is_over(layout.bounds())
        {
            let state = tree.state.downcast_mut::<State>();
            state.open = !state.open;
            if state.open {
                state.query.clear();
                state.needs_focus = true;
                state.picked.set(false);
            }
            shell.capture_event();
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;

        let bounds = layout.bounds();
        let palette = theme.extended_palette();
        let state = tree.state.downcast_ref::<State>();
        let active = state.open || cursor.is_over(bounds);

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    radius: radius().into(),
                    ..Border::default()
                },
                ..Quad::default()
            },
            Background::Color(if active {
                palette.background.weak.color
            } else {
                Color::TRANSPARENT
            }),
        );

        let text_color = palette.background.base.text;

        if self.compact {
            let tint = if active {
                text_color
            } else {
                text_color.scale_alpha(0.7)
            };
            let icon = Rectangle {
                x: bounds.x + (bounds.width - COMPACT_ICON_SIZE) / 2.0,
                y: bounds.y + (bounds.height - COMPACT_ICON_SIZE) / 2.0,
                width: COMPACT_ICON_SIZE,
                height: COMPACT_ICON_SIZE,
            };
            iced::advanced::svg::Renderer::draw_svg(
                renderer,
                iced::advanced::svg::Svg::new(svg::Handle::from_memory(ICON_PANE)).color(tint),
                icon,
                bounds,
            );
            return;
        }

        renderer.fill_text(
            text::Text {
                content: self.current.title().to_owned(),
                bounds: Size::new(bounds.width, bounds.height),
                size: self.text_size.into(),
                line_height: text::LineHeight::default(),
                font: renderer.default_font(),
                align_x: text::Alignment::Center,
                align_y: Vertical::Center,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::None,
            },
            Point::new(
                bounds.x + bounds.width / 2.0,
                bounds.y + bounds.height / 2.0,
            ),
            text_color,
            bounds,
        );
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State>();
        if !state.open {
            return None;
        }

        if state.built_for.as_ref() != Some(&state.query) {
            let content = build_panel(&state.query, self.current);
            let panel_tree = state.panel_tree.get_or_insert_with(|| Tree::new(&content));
            panel_tree.diff(&content);
            state.content = Some(content);
            state.built_for = Some(state.query.clone());
        }

        let position = layout.position() + translation;
        let bounds = layout.bounds();

        Some(overlay::Element::new(Box::new(PickerOverlay {
            state,
            on_select: self.on_select.as_ref(),
            mapper: None,
            anchor: Rectangle {
                x: position.x,
                y: position.y,
                width: bounds.width,
                height: bounds.height,
            },
        })))
    }
}

fn build_panel<'a>(query: &str, current: PaneKind) -> Element<'a, Op, Theme, Renderer> {
    let search = text_input(SEARCH_PLACEHOLDER, query)
        .id(SEARCH_ID)
        .on_input(Op::Query)
        .size(SEARCH_TEXT_SIZE)
        .padding([SEARCH_V_PAD, ITEM_PADDING_H])
        .style(search_style);

    let body: Element<'a, Op, Theme, Renderer> = if query.is_empty() {
        category_body(current)
    } else {
        let (list, height) = filtered_body(query, current);
        iced::widget::scrollable(
            container(list).padding(iced::Padding::ZERO.right(SCROLLBAR_WIDTH + SCROLLBAR_GUTTER)),
        )
        .width(Length::Fill)
        .height(Length::Fixed(height.min(MAX_LIST_HEIGHT)))
        .direction(Direction::Vertical(
            Scrollbar::new()
                .width(SCROLLBAR_WIDTH)
                .scroller_width(SCROLLBAR_WIDTH)
                .margin(0.0),
        ))
        .into()
    };

    container(column![search, body].spacing(GAP).width(Length::Fill))
        .padding(PADDING)
        .width(Length::Fill)
        .style(panel_style)
        .into()
}

fn category_body<'a>(current: PaneKind) -> Element<'a, Op, Theme, Renderer> {
    let mut list = column![].spacing(ITEM_SPACING).width(Length::Fill);

    for (category, kinds) in PaneKind::by_category() {
        let mut group = column![].spacing(ITEM_SPACING);
        for kind in kinds {
            group = group.push(menu_item(kind.title(), Op::Pick(kind), kind == current));
        }
        list = list.push(
            sub_menu(category.title(), styled_menu(group, SUBMENU_WIDTH)).side(SubMenuSide::Left),
        );
    }

    list.into()
}

fn filtered_body<'a>(query: &str, current: PaneKind) -> (Element<'a, Op, Theme, Renderer>, f32) {
    let matches = PaneKind::search(query);

    if matches.is_empty() {
        let empty = container(text_widget("No panes found").size(ITEM_TEXT_SIZE).style(
            |theme: &Theme| {
                text_widget::Style {
                    color: Some(
                        theme
                            .extended_palette()
                            .background
                            .base
                            .text
                            .scale_alpha(0.5),
                    ),
                }
            },
        ))
        .padding([0.0, ITEM_PADDING_H])
        .height(Length::Fixed(ITEM_HEIGHT))
        .align_y(Vertical::Center)
        .width(Length::Fill);

        return (empty.into(), ITEM_HEIGHT);
    }

    let count = matches.len() as f32;
    let mut list = column![].spacing(ITEM_SPACING).width(Length::Fill);
    for kind in matches {
        list = list.push(item_row_with_category(kind, kind == current));
    }

    (
        list.into(),
        count * ITEM_HEIGHT + (count - 1.0).max(0.0) * ITEM_SPACING,
    )
}

fn item_row_with_category<'a>(kind: PaneKind, selected: bool) -> Element<'a, Op, Theme, Renderer> {
    let label = text_widget(kind.title())
        .size(ITEM_TEXT_SIZE)
        .wrapping(text::Wrapping::None)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(Vertical::Center);

    let hint = text_widget(kind.category().title())
        .size(HEADER_TEXT_SIZE)
        .wrapping(text::Wrapping::None)
        .height(Length::Fill)
        .align_y(Vertical::Center)
        .style(|theme: &Theme| text_widget::Style {
            color: Some(
                theme
                    .extended_palette()
                    .background
                    .base
                    .text
                    .scale_alpha(0.4),
            ),
        });

    item_button(row![label, hint].spacing(GAP).into(), kind, selected)
}

fn item_button(
    content: Element<'_, Op, Theme, Renderer>,
    kind: PaneKind,
    selected: bool,
) -> Element<'_, Op, Theme, Renderer> {
    button(content)
        .width(Length::Fill)
        .height(Length::Fixed(ITEM_HEIGHT))
        .padding([0.0, ITEM_PADDING_H])
        .style(move |theme, status| item_style(theme, status, selected))
        .on_press(Op::Pick(kind))
        .into()
}

fn item_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

    let background = if hovered {
        Some(Background::Color(palette.background.strong.color))
    } else if selected {
        Some(Background::Color(
            palette.primary.base.color.scale_alpha(0.25),
        ))
    } else {
        None
    };

    button::Style {
        background,
        text_color: palette.background.base.text,
        border: Border {
            radius: radius().into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

fn panel_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: radius().into(),
        },
        ..container::Style::default()
    }
}

fn search_style(theme: &Theme, _status: text_input::Status) -> text_input::Style {
    let palette = theme.extended_palette();
    let text_color = palette.background.base.text;
    text_input::Style {
        background: Background::Color(palette.background.base.color),
        border: Border {
            color: palette.primary.base.color,
            width: 1.0,
            radius: radius().into(),
        },
        icon: text_color,
        placeholder: Color {
            a: 0.5,
            ..text_color
        },
        value: text_color,
        selection: palette.primary.base.color.scale_alpha(0.35),
    }
}

struct PickerOverlay<'a, 'b, Message> {
    state: &'b mut State,
    on_select: &'b (dyn Fn(PaneKind) -> Message + 'a),
    #[allow(clippy::type_complexity)]
    mapper: Option<Box<dyn Fn(Op) -> Message + 'b>>,
    anchor: Rectangle,
}

impl<Message: Clone> Overlay<Message, Theme, Renderer> for PickerOverlay<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let (Some(content), Some(panel_tree)) =
            (self.state.content.as_mut(), self.state.panel_tree.as_mut())
        else {
            return layout::Node::new(Size::ZERO);
        };

        let max_height = PADDING + SEARCH_HEIGHT + GAP + MAX_LIST_HEIGHT + PADDING;
        let width = PANEL_WIDTH.min(bounds.width);

        let node = content.as_widget_mut().layout(
            panel_tree,
            renderer,
            &layout::Limits::new(Size::new(width, 0.0), Size::new(width, max_height)),
        );
        let size = node.bounds().size();

        let below = self.anchor.y + self.anchor.height + GAP;
        let above = self.anchor.y - size.height - GAP;
        let y = if below + size.height <= bounds.height {
            below
        } else if above >= 0.0 {
            above
        } else {
            (bounds.height - size.height).max(0.0)
        };

        let preferred_x = self.anchor.x + self.anchor.width - size.width;
        let x = preferred_x.clamp(0.0, (bounds.width - size.width).max(0.0));

        node.move_to(Point::new(x, y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        if let (Some(content), Some(panel_tree)) =
            (self.state.content.as_ref(), self.state.panel_tree.as_ref())
        {
            let viewport = layout.bounds();
            content.as_widget().draw(
                panel_tree, renderer, theme, style, layout, cursor, &viewport,
            );
        }
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        if let (Some(content), Some(panel_tree)) =
            (self.state.content.as_mut(), self.state.panel_tree.as_mut())
        {
            content
                .as_widget_mut()
                .operate(panel_tree, layout, renderer, operation);
        }
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<Message>,
    ) {
        if self.state.picked.get() {
            self.close();
            shell.request_redraw();
            return;
        }

        if self.state.needs_focus {
            self.state.needs_focus = false;
            if let (Some(content), Some(panel_tree)) =
                (self.state.content.as_mut(), self.state.panel_tree.as_mut())
            {
                let mut op = focusable::focus::<()>(iced::advanced::widget::Id::from(SEARCH_ID));
                content
                    .as_widget_mut()
                    .operate(panel_tree, layout, renderer, &mut op);
            }
            shell.request_redraw();
        }

        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Named(Named::Escape),
            ..
        }) = event
        {
            self.close();
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Named(Named::Enter),
            ..
        }) = event
            && !self.state.query.is_empty()
        {
            let matches = PaneKind::search(&self.state.query);
            if let [only] = matches[..] {
                shell.publish((self.on_select)(only));
                self.close();
                shell.capture_event();
                shell.request_redraw();
                return;
            }
        }

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && cursor.position().is_some()
            && !cursor.is_over(layout.bounds())
            && !cursor.is_over(self.anchor)
        {
            self.close();
            shell.request_redraw();
            return;
        }

        let mut ops: Vec<Op> = Vec::new();
        let mut local = Shell::new(&mut ops);
        let viewport = layout.bounds();
        if let (Some(content), Some(panel_tree)) =
            (self.state.content.as_mut(), self.state.panel_tree.as_mut())
        {
            content.as_widget_mut().update(
                panel_tree, event, layout, cursor, renderer, clipboard, &mut local, &viewport,
            );
        }
        if local.is_event_captured() {
            shell.capture_event();
        }
        if local.is_layout_invalid() {
            shell.invalidate_layout();
        }
        if local.are_widgets_invalid() {
            shell.invalidate_widgets();
        }
        shell.request_redraw_at(local.redraw_request());

        if matches!(event, Event::Mouse(mouse::Event::CursorMoved { .. })) {
            shell.request_redraw();
        }

        for op in ops {
            match op {
                Op::Query(query) => {
                    self.state.query = query;
                    shell.request_redraw();
                }
                Op::Pick(kind) => {
                    shell.publish((self.on_select)(kind));
                    self.close();
                    shell.request_redraw();
                }
            }
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let viewport = layout.bounds();
        let interaction = if let (Some(content), Some(panel_tree)) =
            (self.state.content.as_ref(), self.state.panel_tree.as_ref())
        {
            content
                .as_widget()
                .mouse_interaction(panel_tree, layout, cursor, &viewport, renderer)
        } else {
            mouse::Interaction::default()
        };

        if interaction == mouse::Interaction::None && cursor.is_over(viewport) {
            return mouse::Interaction::Idle;
        }

        interaction
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'c>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, Renderer>> {
        let on_select = self.on_select;
        let picked = self.state.picked.clone();
        self.mapper = Some(Box::new(move |op| match op {
            Op::Pick(kind) => {
                picked.set(true);
                on_select(kind)
            }
            Op::Query(_) => unreachable!("fly-out overlays never emit a query op"),
        }));
        let mapper = self.mapper.as_deref().unwrap();

        let (content, panel_tree) = (
            self.state.content.as_mut()?,
            self.state.panel_tree.as_mut()?,
        );
        let child = content.as_widget_mut().overlay(
            panel_tree,
            layout,
            renderer,
            &layout.bounds(),
            Vector::ZERO,
        )?;
        Some(child.map(mapper))
    }
}

impl<Message> PickerOverlay<'_, '_, Message> {
    fn close(&mut self) {
        self.state.open = false;
        self.state.query.clear();
        self.state.content = None;
        self.state.panel_tree = None;
        self.state.built_for = None;
        self.state.picked.set(false);
    }
}

impl<'a, Message: Clone + 'a> From<PanePicker<Message>> for Element<'a, Message, Theme, Renderer> {
    fn from(picker: PanePicker<Message>) -> Self {
        Self::new(picker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generous(title: &str, text_size: f32) -> f32 {
        title.chars().count() as f32 * text_size * 0.62
    }

    #[test]
    fn every_title_fits_the_width_it_is_given() {
        for size in [LABEL_FONT_SIZE, 15.0] {
            for &kind in PaneKind::ALL {
                let budget = PanePicker::<()>::label_width_at(kind, size);
                let needed = generous(kind.title(), size);

                assert!(
                    budget >= needed,
                    "{kind:?} at {size}px gets {budget:.1}px but wants up to {needed:.1}px"
                );
            }
        }
    }

    #[test]
    fn slack_keeps_up_with_a_long_title() {
        for length in [10, 20, 40, 80] {
            let title = "n".repeat(length);
            let advances = length as f32 * LOWER_RATIO;
            let budget = (advances * (1.0 + GLYPH_SLACK_RATIO) + GLYPH_SLACK) * LABEL_FONT_SIZE;

            assert!(
                budget >= generous(&title, LABEL_FONT_SIZE),
                "a {length}-character title gets {budget:.1}px but wants up to {:.1}px",
                generous(&title, LABEL_FONT_SIZE)
            );
        }
    }

    #[test]
    fn a_longer_title_is_given_more_width() {
        let short = PanePicker::<()>::label_width(PaneKind::Empty);
        let long = PanePicker::<()>::label_width(PaneKind::NowPlaying);

        assert!(long > short);
    }

    #[test]
    fn a_larger_font_is_given_more_width() {
        let small = PanePicker::<()>::label_width_at(PaneKind::Visualizer, LABEL_FONT_SIZE);
        let large = PanePicker::<()>::label_width_at(PaneKind::Visualizer, 15.0);

        assert!(large > small);
    }

    #[test]
    fn a_two_word_title_is_not_budgeted_as_though_its_space_were_a_letter() {
        let spaced = PanePicker::<()>::label_width(PaneKind::NowPlaying);
        let solid = PanePicker::<()>::label_width_at(PaneKind::Collections, LABEL_FONT_SIZE);

        assert!(
            spaced < solid,
            "\"Now Playing\" is a character longer than \"Collections\" but should \
             still be narrower, since its space is not a letter"
        );
    }
}
