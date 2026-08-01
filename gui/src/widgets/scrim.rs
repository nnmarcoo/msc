//! A modal backdrop that stops the cursor reaching what it covers.
//!
//! `docs/overlay-cursor.md` describes this hazard for iced `Overlay`s, where
//! answering `Interaction::None` hands the cursor back to the layer beneath. A
//! modal drawn as a `stack!` layer rather than an overlay has the same hole for
//! a different reason: `mouse_area` intercepts *presses*, and nothing about
//! being painted over a widget stops that widget being asked about hover. So the
//! panes behind a scrim would keep lighting up their rows, showing tooltips, and
//! offering resize cursors through a backdrop that is plainly covering them.
//!
//! This wraps the backdrop and answers [`mouse::Interaction::Idle`] for every
//! pixel of it. `Idle` is the plain arrow: it claims the cursor without claiming
//! the pixel is interactive, which is the neutral answer the enum otherwise
//! cannot express. `Pointer` would also claim it, but would tell the user every
//! dead pixel of the backdrop is clickable.
//!
//! Claiming hover is all this does. Presses are still the caller's to handle —
//! the modal wants a press on the backdrop to dismiss it, and a press on the
//! dialog to do nothing — so those stay in the `mouse_area`s that wrap the
//! content this is given.
//!
//! The content is asked first, so a widget inside the modal that does have an
//! opinion — a button wanting `Pointer`, a text field wanting `Text` — still
//! gets it. Only the pixels nothing claimed become `Idle`.
//!
//! Every other method forwards, `size_hint` and the tree methods included. This
//! adds nothing to the layout and holds no state of its own, so the wrapper has
//! to be invisible to both: taking the default `size_hint` would answer from
//! `size` instead of the content, which is right only when a widget's hint and
//! size agree.

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector};

pub struct Scrim<'a, Message> {
    content: Element<'a, Message, Theme, Renderer>,
}

impl<'a, Message> Scrim<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Scrim<'_, Message> {
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
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
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
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
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let interaction = self
            .content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer);

        if interaction == mouse::Interaction::None && cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Idle;
        }

        interaction
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<Scrim<'a, Message>> for Element<'a, Message, Theme, Renderer> {
    fn from(scrim: Scrim<'a, Message>) -> Self {
        Self::new(scrim)
    }
}
