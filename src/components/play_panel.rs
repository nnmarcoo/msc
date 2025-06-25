use egui::{vec2, Context, FontId, Image, Label, RichText, ScrollArea, SidePanel};

use crate::{state::State, widgets::styled_button::StyledButton};
use egui_dnd::dnd;

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct PlayPanel {}

impl PlayPanel {
    pub fn new() -> Self {
        PlayPanel {}
    }

    pub fn show(&mut self, ctx: &Context, app_state: &mut State) {
        if !app_state.show_play_panel {
            return;
        }

        let current_track_ref = app_state.queue.get_track_mut_ref(&app_state.library);

        SidePanel::right("Play panel")
            .max_width(350.)
            .min_width(0.)
            .show(ctx, |ui| {
                if ui.available_width() <= 0. {
                    app_state.show_play_panel = false;
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

                    dnd(ui, "queue").show_vec(
                        &mut app_state.queue.tracks.clone(),
                        |ui, item, handle, state| {
                            ui.horizontal(|ui| {
                                ui.allocate_ui(vec2(ui.available_width() - 16., 16.), |ui| {
                                    if let Some(track_ref) = app_state.library.get(item) {
                                        ui.add(
                                            Label::new(track_ref.value().title.clone()).truncate(),
                                        );
                                    }
                                });

                                handle.ui(ui, |ui| {
                                    ui.add(Label::new(
                                        RichText::new("▩").font(FontId::monospace(16.)),
                                    ));
                                });
                            });
                        },
                    );
                });
            });
    }
}
