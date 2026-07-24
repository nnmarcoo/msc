//! Queue pane state. Content rendering is deferred.

#[derive(Debug, Default)]
pub struct State {
    pub show_history: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleHistory,
}

pub fn update(state: &mut State, message: &Message) {
    match message {
        Message::ToggleHistory => state.show_history = !state.show_history,
    }
}
