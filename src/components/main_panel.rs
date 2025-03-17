use super::playlist_view::PlayListView;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct MainPanel {
    playlist_view: PlayListView,
}

impl MainPanel {
    pub fn new() -> Self {
        MainPanel {
            playlist_view: PlayListView::new(),
        }
    }
    pub fn show(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.playlist_view.show(ui);
        });
    }
}
