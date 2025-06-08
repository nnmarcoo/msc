use egui::{Context, SidePanel};

use crate::state::State;

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct PlayPanel {}

impl PlayPanel {
    pub fn new() -> Self {
        PlayPanel {}
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        SidePanel::right("Play panel").show(ctx, |ui| {
            for (i, track) in state.queue.tracks.iter().enumerate() {
                if i == state.queue.current_index {
                    ui.strong(track.title.clone());
                } else {
                    ui.label(track.title.clone());
                }
            }
            ui.allocate_space(egui::vec2(200., 0.))
        });
    }
}
