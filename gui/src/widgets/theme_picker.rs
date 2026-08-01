//! A theme dropdown that shows each theme rather than naming it.
//!
//! A `pick_list` of theme names asks the user to remember what "Kanagawa Lotus"
//! looks like. This draws three swatches per row — background, primary, and the
//! strong background the panels and hovers use — so the list can be read by eye,
//! which is the only way to choose a theme that actually matters.
//!
//! The trigger is a custom widget rather than a `button` because the list has to
//! open as a true `Overlay`: preferences is a scrollable, and a menu built as a
//! `stack!` layer would be clipped by it and scroll away from its trigger.
//!
//! The list opens below the trigger and flips above when that would run off the
//! bottom, since the theme row sits near the top of Appearance but the window
//! can be short enough for either to be the only fit.
//!
//! The panel `Element` is built once per open and kept in tree state, because
//! building it allocates a row per theme that then has to be diffed into a
//! `Tree`; rebuilding that on every cursor move is what makes an overlay list
//! feel heavy. It is dropped on close so a picker nobody has opened costs
//! nothing.
//!
//! `PickerOverlay::mouse_interaction` claims the cursor anywhere over the panel.
//! iced returns the cursor to the layer beneath whenever an overlay answers
//! `Interaction::None`, so without this the gaps between rows would let the
//! preferences view hover through the open list. See `docs/overlay-cursor.md`.

use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::Renderer as _;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Overlay, Shell, Widget, layout, text};
use iced::alignment::Vertical;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, text as text_widget};
use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Renderer, Size,
    Theme, Vector, mouse, overlay,
};

use crate::config::ALL_THEMES;
use crate::styles::radius;

const TRIGGER_HEIGHT: f32 = 26.0;
const TRIGGER_PADDING_H: f32 = 8.0;

const PANEL_WIDTH: f32 = 220.0;
const PANEL_PADDING: f32 = 6.0;
const GAP: f32 = 4.0;

const ROW_HEIGHT: f32 = 26.0;
const ROW_SPACING: f32 = 2.0;
const ROW_PADDING_H: f32 = 8.0;
const MAX_VISIBLE_ROWS: usize = 12;

const TEXT_SIZE: f32 = 12.0;
const SWATCH_SIZE: f32 = 12.0;
const SWATCH_GAP: f32 = 3.0;

const SCROLLBAR_WIDTH: f32 = 4.0;
const SCROLLBAR_GUTTER: f32 = 4.0;

fn max_list_height() -> f32 {
    ROW_HEIGHT * MAX_VISIBLE_ROWS as f32 + ROW_SPACING * (MAX_VISIBLE_ROWS as f32 - 1.0)
}

#[derive(Default)]
struct State {
    open: bool,
    content: Option<Element<'static, Theme, Theme, Renderer>>,
    panel_tree: Option<Tree>,
    built_for: Option<Theme>,
}

impl State {
    fn close(&mut self) {
        self.open = false;
        self.content = None;
        self.panel_tree = None;
        self.built_for = None;
    }
}

pub struct ThemePicker<Message> {
    selected: Theme,
    on_select: Box<dyn Fn(Theme) -> Message>,
    width: Length,
}

impl<Message> ThemePicker<Message> {
    pub fn new(selected: Theme, on_select: impl Fn(Theme) -> Message + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            width: Length::Fixed(PANEL_WIDTH),
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for ThemePicker<Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Fixed(TRIGGER_HEIGHT),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, Length::Fixed(TRIGGER_HEIGHT))
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
            if state.open {
                state.close();
            } else {
                state.open = true;
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
                    color: if active {
                        palette.primary.base.color
                    } else {
                        palette.background.strong.color
                    },
                    width: 1.0,
                    radius: radius().into(),
                },
                ..Quad::default()
            },
            Background::Color(if active {
                palette.background.weak.color
            } else {
                palette.background.base.color
            }),
        );

        let text_color = palette.background.base.text;
        let middle = bounds.y + bounds.height / 2.0;

        renderer.fill_text(
            text::Text {
                content: self.selected.to_string(),
                bounds: Size::new(bounds.width - TRIGGER_PADDING_H * 2.0, bounds.height),
                size: TEXT_SIZE.into(),
                line_height: text::LineHeight::default(),
                font: renderer.default_font(),
                align_x: text::Alignment::Left,
                align_y: Vertical::Center,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::None,
            },
            Point::new(bounds.x + TRIGGER_PADDING_H, middle),
            text_color,
            bounds,
        );

        let swatch_span = SWATCH_SIZE * 3.0 + SWATCH_GAP * 2.0;
        let swatch_left = bounds.x + bounds.width - TRIGGER_PADDING_H - swatch_span;
        draw_swatches(
            renderer,
            &self.selected,
            Point::new(swatch_left, middle - SWATCH_SIZE / 2.0),
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

        if state.built_for.as_ref() != Some(&self.selected) {
            let content = build_panel(&self.selected);
            let panel_tree = state.panel_tree.get_or_insert_with(|| Tree::new(&content));
            panel_tree.diff(&content);
            state.content = Some(content);
            state.built_for = Some(self.selected.clone());
        }

        let position = layout.position() + translation;
        let bounds = layout.bounds();

        Some(overlay::Element::new(Box::new(PickerOverlay {
            state,
            on_select: self.on_select.as_ref(),
            anchor: Rectangle {
                x: position.x,
                y: position.y,
                width: bounds.width,
                height: bounds.height,
            },
        })))
    }
}

fn draw_swatches(renderer: &mut Renderer, theme: &Theme, origin: Point, clip: Rectangle) {
    use iced::advanced::Renderer as _;

    let palette = theme.extended_palette();
    let colors = [
        palette.background.base.color,
        palette.primary.base.color,
        palette.background.strong.color,
    ];

    for (index, color) in colors.into_iter().enumerate() {
        let swatch = Rectangle {
            x: origin.x + index as f32 * (SWATCH_SIZE + SWATCH_GAP),
            y: origin.y,
            width: SWATCH_SIZE,
            height: SWATCH_SIZE,
        };

        if !clip.intersects(&swatch) {
            continue;
        }

        renderer.fill_quad(
            Quad {
                bounds: swatch,
                border: Border {
                    color: Color::BLACK.scale_alpha(0.2),
                    width: 1.0,
                    radius: (SWATCH_SIZE / 4.0).into(),
                },
                ..Quad::default()
            },
            Background::Color(color),
        );
    }
}

fn swatch<'a>(color: Color) -> Element<'a, Theme, Theme, Renderer> {
    container(Space::new().width(SWATCH_SIZE).height(SWATCH_SIZE))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border {
                color: Color::BLACK.scale_alpha(0.2),
                width: 1.0,
                radius: (SWATCH_SIZE / 4.0).into(),
            },
            ..container::Style::default()
        })
        .into()
}

fn swatches<'a>(theme: &Theme) -> Element<'a, Theme, Theme, Renderer> {
    let palette = theme.extended_palette();

    row![
        swatch(palette.background.base.color),
        swatch(palette.primary.base.color),
        swatch(palette.background.strong.color),
    ]
    .align_y(Vertical::Center)
    .spacing(SWATCH_GAP)
    .into()
}

fn build_panel<'a>(selected: &Theme) -> Element<'a, Theme, Theme, Renderer> {
    let mut list = column![].spacing(ROW_SPACING).width(Length::Fill);

    for candidate in ALL_THEMES {
        let is_selected = candidate == selected;

        let label = text_widget(candidate.to_string())
            .size(TEXT_SIZE)
            .wrapping(text::Wrapping::None)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Vertical::Center);

        list = list.push(
            button(
                row![label, swatches(candidate)]
                    .align_y(Vertical::Center)
                    .spacing(SWATCH_GAP),
            )
            .width(Length::Fill)
            .height(Length::Fixed(ROW_HEIGHT))
            .padding([0.0, ROW_PADDING_H])
            .style(move |theme, status| row_style(theme, status, is_selected))
            .on_press(candidate.clone()),
        );
    }

    let body = scrollable(
        container(list).padding(Padding::ZERO.right(SCROLLBAR_WIDTH + SCROLLBAR_GUTTER)),
    )
    .width(Length::Fill)
    .height(Length::Shrink)
    .direction(Direction::Vertical(
        Scrollbar::new()
            .width(SCROLLBAR_WIDTH)
            .scroller_width(SCROLLBAR_WIDTH)
            .margin(0.0),
    ));

    container(body)
        .padding(PANEL_PADDING)
        .width(Length::Fill)
        .max_height(max_list_height() + PANEL_PADDING * 2.0)
        .style(panel_style)
        .into()
}

fn row_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
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

struct PickerOverlay<'a, 'b, Message> {
    state: &'b mut State,
    on_select: &'b (dyn Fn(Theme) -> Message + 'a),
    anchor: Rectangle,
}

impl<Message: Clone> Overlay<Message, Theme, Renderer> for PickerOverlay<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let (Some(content), Some(panel_tree)) =
            (self.state.content.as_mut(), self.state.panel_tree.as_mut())
        else {
            return layout::Node::new(Size::ZERO);
        };

        let width = self.anchor.width.max(PANEL_WIDTH).min(bounds.width);
        let max_height = max_list_height() + PANEL_PADDING * 2.0;

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

        let x = self
            .anchor
            .x
            .clamp(0.0, (bounds.width - size.width).max(0.0));

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

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<Message>,
    ) {
        if let Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            ..
        }) = event
        {
            self.state.close();
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && cursor.position().is_some()
            && !cursor.is_over(layout.bounds())
            && !cursor.is_over(self.anchor)
        {
            self.state.close();
            shell.request_redraw();
            return;
        }

        let mut picks: Vec<Theme> = Vec::new();
        let mut local = Shell::new(&mut picks);
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

        if let Some(theme) = picks.into_iter().next_back() {
            shell.publish((self.on_select)(theme));
            self.state.close();
            shell.request_redraw();
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
}

impl<'a, Message: Clone + 'a> From<ThemePicker<Message>> for Element<'a, Message, Theme, Renderer> {
    fn from(picker: ThemePicker<Message>) -> Self {
        Self::new(picker)
    }
}
