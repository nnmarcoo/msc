use eframe::egui::{CentralPanel, Context, Ui};

use crate::msc::{State, View};

use super::{library_view::LibraryView, playlist_view::PlaylistView, settings_view::SettingsView};

// track selections break if the user changes the search query
// cache filtered_tracks so it's not calculated every frame

pub struct MainArea {
    library: LibraryView,
    settings: SettingsView,
    playlist: PlaylistView,
}

impl MainArea {
    pub fn new() -> Self {
        MainArea {
            library: LibraryView::new(),
            settings: SettingsView::new(),
            playlist: PlaylistView::new(),
        }
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        CentralPanel::default().show(ctx, |ui| match state.view {
            View::Playlist => self.playlist.show(ctx, ui, state),
            View::_Search => self.show_search(ui, state),
            View::Settings => self.settings.show(ui, state),
            View::Library => self.library.show(ui, state),
        });
    }

    fn show_search(&mut self, ui: &mut Ui, _state: &mut State) {
        ui.heading("Search View");
    }
}
