use egui::{vec2, Context, Image, ScrollArea, SidePanel};

use crate::{state::State, widgets::styled_button::StyledButton};

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct PlayPanel {}

impl PlayPanel {
    pub fn new() -> Self {
        PlayPanel {}
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        if !state.show_play_panel {
            return;
        }

        let current_track_ref = state.queue.get_track_mut_ref(&state.library);

        SidePanel::right("Play panel")
            .max_width(350.)
            .min_width(0.)
            .show(ctx, |ui| {
                if ui.available_width() <= 0. {
                    state.show_play_panel = false;
                    ui.allocate_space(vec2(150., 0.));
                    return;
                }

                ScrollArea::vertical().show(ui, |ui| {
                    ui.strong("Album");

                    if let Some(mut current_track) = current_track_ref {
                        if let Some(texture) = &current_track.texture {
                            ui.add(StyledButton::new(
                                vec2(ui.available_width(), ui.available_width()),
                                &Image::new(texture),
                                || {},
                            ));
                        } else {
                            // Should I fill the space with a spinner or image?
                            current_track.load_texture(ui.ctx());
                        }
                    } else {
                        ui.label("No track playing");
                    }

                    ui.separator();

                    for (i, hash) in state.queue.tracks.iter().enumerate() {
                        if let Some(track_ref) = state.library.get(hash) {
                            let track = track_ref.value();

                            if i == state.queue.current_index {
                                ui.strong(track.title.clone());
                            } else {
                                ui.label(track.title.clone());
                            }
                        }
                    }
                });
            });
    }
}
