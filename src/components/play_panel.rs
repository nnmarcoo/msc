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

        let current_track = state.queue.get_current_track();

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

                    if let Some(texture) = current_track.image.get_texture_handle() {
                        ui.add(StyledButton::new(
                            vec2(ui.available_width(), ui.available_width()),
                            &Image::new(&texture),
                            || {},
                        ));
                    } else {
                        current_track.image.load(ui.available_width() as u32, ctx);
                    }

                    ui.separator();

                    for (i, track) in state.queue.tracks.iter().enumerate() {
                        if i == state.queue.current_index {
                            ui.strong(track.title.clone());
                        } else {
                            ui.label(track.title.clone());
                        }
                    }
                });
            });
    }
}
