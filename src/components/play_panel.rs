use egui::{
    scroll_area::ScrollBarVisibility, vec2, Color32, Context, CursorIcon, Frame, Image, Label,
    RichText, ScrollArea, SidePanel, Stroke,
};

use crate::{state::State, widgets::styled_button::StyledButton};
use egui_dnd::dnd;

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
                        ui.strong("Album");

                        if let Some(mut current_track) =
                            app_state.queue.get_track_mut_ref(&app_state.library)
                        {
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

                        let mut hovered_any = false;

                        dnd(ui, "queue").show_vec(
                            &mut app_state.queue.tracks,
                            |ui, item, handle, state| {
                                let hovered = Some(state.index) == self.hover_idx;
                                if Frame::group(ui.style())
                                    .stroke(Stroke::NONE)
                                    .fill(if hovered {
                                        Color32::from_rgb(40, 40, 40)
                                    } else {
                                        Color32::TRANSPARENT
                                    })
                                    .show(ui, |ui| {
                                        if handle
                                            .ui(ui, |ui| {
                                                if let Some(track_ref) = app_state.library.get(item)
                                                {
                                                    ui.vertical(|ui| {
                                                        ui.add(
                                                            Label::new(
                                                                RichText::new(
                                                                    track_ref.value().title.clone(),
                                                                )
                                                                .strong(),
                                                            )
                                                            .truncate(),
                                                        );

                                                        ui.add(
                                                            Label::new(
                                                                track_ref.value().artist.clone(),
                                                            )
                                                            .truncate(),
                                                        );
                                                    });
                                                    ui.allocate_space(vec2(
                                                        ui.available_width(),
                                                        0.,
                                                    ));
                                                }
                                            })
                                            .hovered()
                                        {
                                            ctx.set_cursor_icon(CursorIcon::Default);
                                        }
                                    })
                                    .response
                                    .hovered()
                                {
                                    self.hover_idx = Some(state.index);
                                    hovered_any = true;
                                }
                            },
                        );

                        if !hovered_any {
                            self.hover_idx = None;
                        }
                    });
            });
    }
}
