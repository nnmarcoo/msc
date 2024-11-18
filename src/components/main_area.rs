use eframe::egui::{CentralPanel, Context, Image, ScrollArea, Ui};

use crate::msc::{State, View};

pub struct MainArea {}

impl MainArea {
    pub fn new() -> Self {
        MainArea {}
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        CentralPanel::default().show(ctx, |ui| match state.view {
            View::Playlist => self.show_playlist(ui, state),
            View::Search => self.show_search(ui, state),
            View::Settings => self.show_settings(ui, state),
            View::Library => self.show_library(ui, state),
        });
    }

    fn show_playlist(&mut self, ui: &mut Ui, state: &mut State) {
        ui.heading("Playlist View");
    }

    fn show_search(&mut self, ui: &mut Ui, state: &mut State) {
        ui.heading("Search View");
    }

    fn show_settings(&mut self, ui: &mut Ui, state: &mut State) {
        ui.heading("Settings View");
    }

    fn show_library(&mut self, ui: &mut Ui, state: &mut State) {
        ui.heading("Library View");
        ScrollArea::vertical().show(ui, |ui| {
            //ui.label(state.library.to_string());
            for s in state.library.tracks.iter_mut() {
                if let Some(handle) = &s.image {
                    let img = Image::new(handle);
                    ui.add(img);
                }
            }
        });
    }
}
