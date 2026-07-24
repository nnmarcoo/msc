//! Library pane state. Content rendering is deferred; only the per-pane search
//! query is modelled for now.

#[derive(Debug, Default)]
pub struct State {
    pub search: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
}

pub fn update(state: &mut State, message: Message) {
    let Message::SearchChanged(query) = message;
    state.search = query;
}
