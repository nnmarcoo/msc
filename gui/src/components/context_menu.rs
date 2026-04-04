use iced::widget::column;
use iced::{Element, Length};
use iced_aw::ContextMenu;

use crate::widgets::menu::{menu_item, menu_separator, styled_menu};

pub enum MenuElement<Message> {
    Button { label: String, message: Message },
    Separator,
}

impl<Message: Clone> MenuElement<Message> {
    pub fn button(label: impl Into<String>, message: Message) -> Self {
        Self::Button {
            label: label.into(),
            message,
        }
    }
}

pub fn context_menu<'a, Message: 'a + Clone + 'static>(
    content: impl Into<Element<'a, Message>>,
    items: Vec<MenuElement<Message>>,
    width: Length,
) -> Element<'a, Message> {
    ContextMenu::new(content, move || {
        let menu_column = items.iter().fold(column![].spacing(2), |col, item| {
            let element: Element<'a, Message> = match item {
                MenuElement::Button { label, message } => {
                    menu_item(label.as_str(), message.clone())
                }
                MenuElement::Separator => menu_separator(),
            };
            col.push(element)
        });

        styled_menu(menu_column)
    })
    .into()
}
