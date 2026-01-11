use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse;
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::{self, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::event::Event;
use iced::{Border, Color, Element, Length, Rectangle, Shadow, Size, Theme};

pub struct CanvasButton<'a, Message> {
    content: Element<'a, Message>,
    on_press: Option<Message>,
    width: Length,
    height: Length,
    padding: f32,
}

#[derive(Default)]
pub struct State {
    is_hovered: bool,
    is_pressed: bool,
}

pub fn canvas_button<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> CanvasButton<'a, Message> {
    CanvasButton {
        content: content.into(),
        on_press: None,
        width: Length::Shrink,
        height: Length::Shrink,
        padding: 0.,
    }
}

impl<'a, Message> CanvasButton<'a, Message> {
    pub fn on_press(mut self, msg: Message) -> Self {
        self.on_press = Some(msg);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

impl<'a, Message: Clone> Widget<Message, Theme, iced::Renderer> for CanvasButton<'a, Message> {
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);

        let content_layout =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &limits);

        let padding = self.padding * 2.0;
        let content_size = content_layout.size();
        let button_size = limits.resolve(
            self.width,
            self.height,
            content_size.expand(Size::new(padding, padding)),
        );

        layout::Node::with_children(button_size, vec![content_layout])
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let is_mouse_over = cursor.is_over(bounds);

        let palette = theme.extended_palette();

        let background_color = if state.is_pressed && is_mouse_over {
            palette.primary.strong.color
        } else if state.is_hovered && is_mouse_over {
            palette.primary.base.color
        } else {
            Color::TRANSPARENT
        };

        if background_color != Color::TRANSPARENT {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        radius: 0.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                    shadow: Shadow::default(),
                    snap: true,
                },
                background_color,
            );
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().unwrap(),
            cursor,
            viewport,
        );
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        operation.container(None, layout.bounds());
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if self.on_press.is_none() {
            return;
        }

        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        let is_mouse_over = cursor.is_over(bounds);

        match *event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                state.is_hovered = is_mouse_over;
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if is_mouse_over {
                    state.is_pressed = true;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_pressed {
                    state.is_pressed = false;

                    if is_mouse_over {
                        if let Some(on_press) = self.on_press.clone() {
                            shell.publish(on_press);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let is_mouse_over = cursor.is_over(layout.bounds());

        if is_mouse_over && self.on_press.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, Message: 'a + Clone> From<CanvasButton<'a, Message>> for Element<'a, Message> {
    fn from(button: CanvasButton<'a, Message>) -> Self {
        Element::new(button)
    }
}
