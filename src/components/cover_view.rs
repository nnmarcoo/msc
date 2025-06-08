use std::cmp::max;

use egui::{
    scroll_area::ScrollBarVisibility, vec2, Color32, Context, CursorIcon, Image, Label, Rect,
    RichText, ScrollArea, Spinner, Ui,
};

use crate::{
    core::playlist::Playlist,
    state::{State, View},
    widgets::link_label::link_label,
};

use super::shared::show_empty_library;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct CoverView {
    expanded_index: Option<usize>,
}

impl CoverView {
    pub fn new() -> Self {
        CoverView {
            expanded_index: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, ctx: &Context, state: &mut State) {}
}
