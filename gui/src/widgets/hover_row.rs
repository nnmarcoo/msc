//! A settings row that answers the pointer: bars at both ends, and a slot that
//! holds a control only while the cursor is over it.
//!
//! It is a widget rather than a `container` with a style function because both
//! behaviours need hover state during `draw`, and a style function is handed a
//! `Status` for the widget it styles, not for a row it happens to sit inside. A
//! `mouse_area` could report the hover, but only by routing it through a message
//! and a rebuild, which is a frame late and puts per-row cursor state in the
//! application. Hover is local, so it lives in the widget's own tree state.
//!
//! The hover slot is laid out whether or not it is drawn, so the row keeps one
//! width and its label stops in the same place both ways. Revealing a control by
//! reflowing everything beside it is the failure this avoids: the text would
//! shift as the cursor arrives, which reads as the row flinching away.
//!
//! `size` delegates to the label rather than filling, because a settings list
//! wants rows as tall as their content; the row then centres a shorter trailing
//! control against a taller label rather than stretching it.
//!
//! Children are ordered label, trailing, hover, which is what `hover_index`
//! computes. The order matters only here and in `draw`, where the hover child is
//! skipped while the cursor is elsewhere — every other pass walks all of them,
//! so a hidden control still lays out, still updates, and still answers for the
//! cursor, which is what lets it be pressed the instant it appears.

use iced::advanced::widget::{Tree, tree};
use iced::advanced::{self, Clipboard, Layout, Shell, Widget, layout, overlay};
use iced::{
    Background, Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Vector, mouse,
};

use crate::styles::RULE_HEIGHT;

const BAR_WIDTH: f32 = RULE_HEIGHT / 2.0;
const BAR_GAP: f32 = 8.0;

pub struct HoverRow<'a, Message> {
    children: Vec<Element<'a, Message>>,
    has_trailing: bool,
    slot: f32,
    has_hover: bool,
}

impl<'a, Message> HoverRow<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            children: vec![content.into()],
            has_trailing: false,
            slot: 0.0,
            has_hover: false,
        }
    }

    pub fn trailing(mut self, trailing: impl Into<Element<'a, Message>>) -> Self {
        self.children.insert(1, trailing.into());
        self.has_trailing = true;
        self
    }

    pub fn hover_slot(mut self, width: f32, element: Option<Element<'a, Message>>) -> Self {
        self.slot = width;
        self.has_hover = element.is_some();
        if let Some(element) = element {
            self.children.push(element);
        }
        self
    }

    fn hover_index(&self) -> Option<usize> {
        self.has_hover.then_some(1 + usize::from(self.has_trailing))
    }
}

#[derive(Default)]
struct State {
    hovered: bool,
}

impl<Message> Widget<Message, iced::Theme, Renderer> for HoverRow<'_, Message>
where
    Message: Clone,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        self.children[0].as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let inset = BAR_WIDTH + BAR_GAP;
        let slot = if self.slot > 0.0 {
            self.slot + BAR_GAP
        } else {
            0.0
        };

        let trailing = self.has_trailing.then(|| {
            let loose = layout::Limits::new(Size::ZERO, limits.max());
            self.children[1]
                .as_widget_mut()
                .layout(&mut tree.children[1], renderer, &loose)
        });
        let trailing_width = trailing
            .as_ref()
            .map_or(0.0, |node| node.size().width + BAR_GAP);

        let inner_limits = limits.shrink(Size::new(inset * 2.0 + slot + trailing_width, 0.0));
        let label = self.children[0]
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &inner_limits)
            .move_to((inset, 0.0));
        let label_size = label.size();

        let hover = self.hover_index().map(|index| {
            let limits = layout::Limits::new(Size::ZERO, Size::new(self.slot, limits.max().height));
            self.children[index].as_widget_mut().layout(
                &mut tree.children[index],
                renderer,
                &limits,
            )
        });

        let mut height = label_size.height;
        for node in trailing.iter().chain(hover.iter()) {
            height = height.max(node.size().height);
        }

        let centred = |node: layout::Node, x: f32| {
            let y = ((height - node.size().height) / 2.0).round();
            node.move_to((x, y))
        };

        let mut nodes = vec![label];
        if let Some(node) = trailing {
            let x = inset + label_size.width + slot + BAR_GAP;
            nodes.push(centred(node, x));
        }
        if let Some(node) = hover {
            let x = inset + label_size.width + BAR_GAP + ((self.slot - node.size().width) / 2.0);
            nodes.push(centred(node, x.round()));
        }

        layout::Node::with_children(
            Size::new(
                label_size.width + inset * 2.0 + slot + trailing_width,
                height,
            ),
            nodes,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &iced::Theme,
        style: &advanced::renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let hovered = tree.state.downcast_ref::<State>().hovered;

        if hovered {
            let color = theme.extended_palette().primary.base.color;
            let bar = |x: f32| advanced::renderer::Quad {
                bounds: Rectangle {
                    x,
                    y: bounds.y,
                    width: BAR_WIDTH,
                    height: bounds.height,
                },
                border: Border {
                    radius: 0.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            };
            advanced::Renderer::fill_quad(renderer, bar(bounds.x), Background::Color(color));
            advanced::Renderer::fill_quad(
                renderer,
                bar(bounds.x + bounds.width - BAR_WIDTH),
                Background::Color(color),
            );
        }

        let hover_index = self.hover_index();
        for (index, ((child, state), child_layout)) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
        {
            if Some(index) == hover_index && !hovered {
                continue;
            }
            child.as_widget().draw(
                state,
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );
        }
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
        let hovered = cursor.is_over(layout.bounds());
        let state = tree.state.downcast_mut::<State>();
        if state.hovered != hovered {
            state.hovered = hovered;
            shell.request_redraw();
        }

        for ((child, child_tree), child_layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                child_tree,
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), child_layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, child_layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn advanced::widget::Operation,
    ) {
        for ((child, child_tree), child_layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child
                .as_widget_mut()
                .operate(child_tree, child_layout, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<HoverRow<'a, Message>> for Element<'a, Message> {
    fn from(row: HoverRow<'a, Message>) -> Self {
        Element::new(row)
    }
}
