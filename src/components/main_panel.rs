use egui::{CentralPanel, Context};

use crate::structs::{State, View};

use super::{loading_view::LoadingView, playlist_view::PlayListView, settings_view::SettingsView};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct MainPanel {
    playlist_view: PlayListView,
    settings_view: SettingsView,
    loading_view: LoadingView,
}

impl MainPanel {
    pub fn new() -> Self {
        MainPanel {
            playlist_view: PlayListView::new(),
            settings_view: SettingsView::new(),
            loading_view: LoadingView::new(),
        }
    }
    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        CentralPanel::default().show(ctx, |ui| match state.view {
            View::Playlist => self.playlist_view.show(ui, ctx, state),
            View::Settings => self.settings_view.show(ui, state),
            View::Loading => self.loading_view.show(ui, state),
        });
    }
}
