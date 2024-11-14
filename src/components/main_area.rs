use eframe::egui::{CentralPanel, Context};

use crate::msc::{State, View};

pub struct MainArea {}

impl MainArea {
    pub fn new() -> Self {
        MainArea {}
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        CentralPanel::default().show(ctx, |ui| match state.view {
            View::Playlist => ui.label("Playlist"),
            View::Search => ui.label("Search"),
            View::Settings => ui.label("Settings"),
            View::Library => ui.label("Library"),
        });
    }
}
