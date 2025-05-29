use egui::{CentralPanel, Context};

use crate::structs::{State, View};

use super::{
    library_view::LibraryView, playlists_view::PlayListsView, settings_view::SettingsView,
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct MainPanel {
    playlist_view: PlayListsView,
    settings_view: SettingsView,
    library_view: LibraryView,
}

impl MainPanel {
    pub fn new() -> Self {
        MainPanel {
            playlist_view: PlayListsView::new(),
            settings_view: SettingsView::new(),
            library_view: LibraryView::new(),
        }
    }
    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        CentralPanel::default().show(ctx, |ui| match state.view {
            View::Playlists => self.playlist_view.show(ui, ctx, state),
            View::Settings => self.settings_view.show(ui, state),
            View::Library => self.library_view.show(ui, state),
        });
    }
}
