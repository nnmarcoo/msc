use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct PlayPanel {}

impl PlayPanel {
    pub fn new() -> Self {
        PlayPanel {}
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("Play panel").show(ctx, |ui| {
            ui.label("Play panel");
            ui.allocate_space(egui::vec2(200., 0.))
        });
    }
}
