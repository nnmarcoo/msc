use egui::{
    scroll_area::ScrollBarVisibility, vec2, Color32, Context, CursorIcon, Frame, Image, Label,
    RichText, ScrollArea, SidePanel, Stroke, TextWrapMode,
};

use crate::{state::State, widgets::styled_button::StyledButton};
use egui_dnd::{dnd, DragDropItem};

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct PlayPanel {
    hover_idx: Option<usize>,
}

impl PlayPanel {
    pub fn new() -> Self {
        PlayPanel { hover_idx: None }
    }

    pub fn show(&mut self, ctx: &Context, app_state: &mut State) {
        if !app_state.show_play_panel {
            return;
        }

        SidePanel::right("Play panel")
            .max_width(350.)
            .min_width(0.)
            .show(ctx, |ui| {
                if ui.available_width() <= 0. {
                    app_state.show_play_panel = false;
                    ui.allocate_space(vec2(150., 0.));
                    return;
                }

                ScrollArea::vertical()
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        if let Some(mut current_track) =
                            app_state.queue.get_track_mut_ref(&app_state.library)
                        {
                            let image_size = vec2(ui.available_width(), ui.available_width());
                            if let Some(texture) = &current_track.texture {
                                ui.add(StyledButton::new(image_size, &Image::new(texture), || {}));
                            } else {
                                ui.allocate_space(image_size);
                                current_track.load_texture(ui.ctx());
                            }
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.add(
                                    Label::new("Nothing is playing")
                                        .wrap_mode(TextWrapMode::Extend),
                                );
                            });
                            return;
                        }

                        ui.separator();

                        let mut hovered_any = false;
                        dnd(ui, "queue").show_custom_vec(
                            &mut app_state.queue.tracks,
                            |ui, tracks, iter| {
                                tracks.iter().enumerate().for_each(|(i, track)| {
                                    if i < app_state.queue.current_index + 1 {
                                        return;
                                    }
                                    iter.next(ui, track.id(), i, true, |ui, track_handle| {
                                        track_handle.ui(ui, |ui, handle, state| {
                                            if handle
                                                .ui(ui, |ui| {
                                                    Frame::group(ui.style())
                                                        .fill(
                                                            if Some(state.index) != self.hover_idx {
                                                                Color32::TRANSPARENT
                                                            } else {
                                                                Color32::from_rgb(40, 40, 40)
                                                            },
                                                        )
                                                        .stroke(Stroke::NONE)
                                                        .show(ui, |ui| {
                                                            if let Some(track_ref) =
                                                                app_state.library.get(track)
                                                            {
                                                                ui.vertical(|ui| {
                                                                    ui.add(
                                                                        Label::new(
                                                                            RichText::new(
                                                                                track_ref
                                                                                    .value()
                                                                                    .title
                                                                                    .clone(),
                                                                            )
                                                                            .strong(),
                                                                        )
                                                                        .truncate(),
                                                                    );

                                                                    ui.add(
                                                                        Label::new(
                                                                            track_ref
                                                                                .value()
                                                                                .artist
                                                                                .clone(),
                                                                        )
                                                                        .truncate(),
                                                                    );
                                                                });
                                                                ui.allocate_space(vec2(
                                                                    ui.available_width(),
                                                                    0.,
                                                                ));
                                                            }
                                                        });
                                                })
                                                .hovered()
                                            {
                                                ctx.set_cursor_icon(CursorIcon::default());
                                                self.hover_idx = Some(i);
                                                hovered_any = true;
                                            }
                                        })
                                    });
                                });
                            },
                        );
                        if !hovered_any {
                            self.hover_idx = None;
                        }
                    });
            });
    }
}
