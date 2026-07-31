//! Right-click menus.
//!
//! A custom widget because the panel has to escape its pane's clip bounds: a
//! menu opened near the bottom of a short pane is taller than the pane itself,
//! so it must be a true `Overlay` rather than a `stack!` layer.
//!
//! The panel and its `Tree` live in [`State`] and are rebuilt only when the
//! labels change (`built_for`). This is load-bearing, not an optimization: a
//! menu row keeps its hover flag in tree state, so building a fresh `Tree` each
//! frame, as this first did, discards that flag every frame and the rows never
//! light up or register a press. Presses travel back through a local `Shell` as
//! indices into the item list, which keeps the widget generic over `Message`
//! without cloning the caller's messages on every frame.
//!
//! `armed` swallows exactly one mouse release after opening. The right-press
//! that opens the menu is followed by its own release, and the panel appears
//! under the cursor, so that release would otherwise land on whichever item sits
//! at the corner and fire it before the menu was ever seen.
//!
//! Labels are owned rather than borrowed: a menu says "Queue 3 tracks", which
//! is built per frame from the selection and has nowhere to live otherwise.
//! [`Entry::Separator`] is a row rather than a separate concept so that
//! authoring a menu stays a single flat list in visual order.
//!
//! The opening right-press is forwarded to the wrapped content *before* the
//! menu opens, and only then captured. Order matters: the content is what knows
//! which row was hit, and a menu that swallowed the press outright would leave
//! the selection describing wherever the last left-click landed. Forwarding it
//! first lets a track list report the row, so the app can apply Explorer's rule
//! (a right-click inside the selection keeps it, one outside replaces it)
//! before any entry is chosen. `on_open` is the alternative for content that
//! cannot report a position itself.
//!
//! The panel flips rather than clamps when it would run off screen, so the
//! cursor never lands on top of an item it might immediately activate. When no
//! menu of its own is open the widget still forwards `overlay` to the content it
//! wraps, so wrapping something never disables that thing's own overlays.
//!
//! [`MenuOverlay::mouse_interaction`] claims the cursor over every pixel the
//! panel covers, not just the interactive ones. iced hands the cursor back to
//! the layer beneath whenever an overlay answers `Interaction::None`, so the
//! padding and the gaps between items would otherwise let the pane underneath
//! hover through the menu. See `docs/overlay-cursor.md`.

use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Overlay, Shell, Widget, layout, renderer};
use iced::widget::column;
use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse, overlay,
};

use crate::widgets::menu::{CONTAINER_PADDING, ITEM_HEIGHT, menu_item, styled_menu};

const MENU_WIDTH: f32 = 190.0;
const ITEM_SPACING: f32 = 2.0;
const SEPARATOR_HEIGHT: f32 = 7.0;

pub enum Entry<Message> {
    Button { label: String, message: Message },
    Separator,
}

impl<Message> Entry<Message> {
    pub fn button(label: impl Into<String>, message: Message) -> Self {
        Self::Button {
            label: label.into(),
            message,
        }
    }

    fn height(&self) -> f32 {
        match self {
            Self::Button { .. } => ITEM_HEIGHT,
            Self::Separator => SEPARATOR_HEIGHT,
        }
    }
}

#[derive(Default)]
struct State {
    open: Option<Point>,
    armed: bool,
    panel: Option<Element<'static, usize, Theme, Renderer>>,
    panel_tree: Option<Tree>,
    built_for: Option<Vec<String>>,
}

pub struct ContextMenu<'a, Message> {
    content: Element<'a, Message, Theme, Renderer>,
    entries: Vec<Entry<Message>>,
    on_open: Option<Message>,
}

impl<'a, Message> ContextMenu<'a, Message> {
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        entries: Vec<Entry<Message>>,
    ) -> Self {
        Self {
            content: content.into(),
            entries,
            on_open: None,
        }
    }

    pub fn on_open(mut self, message: Message) -> Self {
        self.on_open = Some(message);
        self
    }

    fn panel_size(&self) -> Size {
        let height: f32 = self.entries.iter().map(Entry::height).sum::<f32>()
            + ITEM_SPACING * (self.entries.len().saturating_sub(1)) as f32
            + CONTAINER_PADDING * 2.0;
        Size::new(MENU_WIDTH, height)
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for ContextMenu<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.content]);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(node.size(), vec![node])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let opening = matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
        )
        .then(|| cursor.position_over(layout.bounds()))
        .flatten();

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap_or(layout),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if let Some(position) = opening {
            let state = tree.state.downcast_mut::<State>();
            state.open = Some(position);
            state.armed = false;
            if let Some(message) = self.on_open.clone() {
                shell.publish(message);
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
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().unwrap_or(layout),
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap_or(layout),
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State>();

        let Some(position) = state.open else {
            return self.content.as_widget_mut().overlay(
                &mut tree.children[0],
                layout.children().next().unwrap_or(layout),
                renderer,
                viewport,
                translation,
            );
        };

        let labels: Vec<String> = self
            .entries
            .iter()
            .map(|entry| match entry {
                Entry::Button { label, .. } => label.clone(),
                Entry::Separator => String::new(),
            })
            .collect();

        if state.built_for.as_ref() != Some(&labels) {
            let mut items = column![].spacing(ITEM_SPACING);
            for (index, entry) in self.entries.iter().enumerate() {
                items = items.push(match entry {
                    Entry::Button { label, .. } => menu_item(label.clone(), index, false),
                    Entry::Separator => separator(),
                });
            }

            let panel = styled_menu(items, MENU_WIDTH);
            let panel_tree = state.panel_tree.get_or_insert_with(|| Tree::new(&panel));
            panel_tree.diff(&panel);
            state.panel = Some(panel);
            state.built_for = Some(labels);
        }

        Some(overlay::Element::new(Box::new(MenuOverlay {
            state,
            entries: &self.entries,
            position: position + translation,
            size: self.panel_size(),
        })))
    }
}

fn separator<'a>() -> Element<'a, usize, Theme, Renderer> {
    iced::widget::container(iced::widget::Space::new())
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(|theme: &Theme| iced::widget::container::Style {
            background: Some(
                theme
                    .extended_palette()
                    .background
                    .strong
                    .color
                    .scale_alpha(0.6)
                    .into(),
            ),
            ..Default::default()
        })
        .into()
}

struct MenuOverlay<'b, Message> {
    state: &'b mut State,
    entries: &'b [Entry<Message>],
    position: Point,
    size: Size,
}

impl<Message: Clone> Overlay<Message, Theme, Renderer> for MenuOverlay<'_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let (Some(panel), Some(panel_tree)) =
            (self.state.panel.as_mut(), self.state.panel_tree.as_mut())
        else {
            return layout::Node::new(Size::ZERO);
        };

        let node = panel.as_widget_mut().layout(
            panel_tree,
            renderer,
            &layout::Limits::new(Size::ZERO, self.size),
        );

        let size = node.size();

        let x = if self.position.x + size.width <= bounds.width {
            self.position.x
        } else {
            (self.position.x - size.width).max(0.0)
        };
        let y = if self.position.y + size.height <= bounds.height {
            self.position.y
        } else {
            (self.position.y - size.height).max(0.0)
        };

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
        let viewport = layout.bounds();
        if let (Some(panel), Some(panel_tree)) =
            (self.state.panel.as_ref(), self.state.panel_tree.as_ref())
        {
            panel.as_widget().draw(
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
        if let Event::Mouse(mouse::Event::ButtonPressed(_)) = event
            && !cursor.is_over(layout.bounds())
        {
            self.state.open = None;
            shell.request_redraw();
            return;
        }

        if let Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            ..
        }) = event
        {
            self.state.open = None;
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        if let Event::Mouse(mouse::Event::ButtonReleased(_)) = event
            && !self.state.armed
        {
            self.state.armed = true;
            shell.capture_event();
            return;
        }

        let mut picked: Vec<usize> = Vec::new();
        let mut local = Shell::new(&mut picked);
        let viewport = layout.bounds();

        let (Some(panel), Some(panel_tree)) =
            (self.state.panel.as_mut(), self.state.panel_tree.as_mut())
        else {
            return;
        };

        panel.as_widget_mut().update(
            panel_tree, event, layout, cursor, renderer, clipboard, &mut local, &viewport,
        );

        if local.is_event_captured() {
            shell.capture_event();
        }
        shell.request_redraw_at(local.redraw_request());

        if matches!(event, Event::Mouse(mouse::Event::CursorMoved { .. })) {
            shell.request_redraw();
        }

        for index in picked {
            if let Some(Entry::Button { message, .. }) = self.entries.get(index) {
                shell.publish(message.clone());
            }
            self.state.open = None;
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
        let interaction = match (self.state.panel.as_ref(), self.state.panel_tree.as_ref()) {
            (Some(panel), Some(panel_tree)) => panel
                .as_widget()
                .mouse_interaction(panel_tree, layout, cursor, &viewport, renderer),
            _ => mouse::Interaction::None,
        };

        if interaction == mouse::Interaction::None && cursor.is_over(viewport) {
            return mouse::Interaction::Idle;
        }

        interaction
    }
}

impl<'a, Message: Clone + 'a> From<ContextMenu<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
{
    fn from(menu: ContextMenu<'a, Message>) -> Self {
        Self::new(menu)
    }
}
