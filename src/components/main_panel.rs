use egui::{CentralPanel, Context};

use crate::structs::{State, View};

use super::{
    library_view::LibraryView, cover_view::CoverView, settings_view::SettingsView,
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct MainPanel {
    cover_view: CoverView,
    settings_view: SettingsView,
    library_view: LibraryView,
}

impl MainPanel {
    pub fn new() -> Self {
        MainPanel {
            cover_view: CoverView::new(),
            settings_view: SettingsView::new(),
            library_view: LibraryView::new(),
        }
    }
    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        CentralPanel::default().show(ctx, |ui| match state.view {
            View::Covers => self.cover_view.show(ui, ctx, state),
            View::Settings => self.settings_view.show(ui, state),
            View::Library => self.library_view.show(ui, state),
        });
    }
}
