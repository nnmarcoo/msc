use eframe::egui::{
    include_image, scroll_area::ScrollBarVisibility, Context, ImageButton, ScrollArea, SidePanel,
};

use crate::{
    backend::playlist::Playlist,
    msc::{State, View},
};

pub struct AudioColumn {}

impl AudioColumn {
    pub fn new() -> Self {
        AudioColumn {}
    }

    pub fn show(&mut self, ctx: &Context, state: &mut State) {
        SidePanel::left("audio_column")
            .resizable(false)
            .exact_width(64.)
            .show_separator_line(false)
            .show(ctx, |ui| {
                if ui
                    .add_sized(
                        [48., 48.],
                        ImageButton::new(include_image!("../../assets/icons/library.png"))
                            .rounding(5.),
                    )
                    .on_hover_text("Library")
                    .clicked()
                {
                    state.view = View::Library;
                }

                let has_playlists = !state.config.playlists.is_empty();

                if has_playlists {
                    ui.add_space(10.);
                }

                ScrollArea::vertical()
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                    .max_height(ui.available_height() - 64.)
                    .show(ui, |ui| {
                        let mut to_remove = None;

                        for (i, playlist) in state.config.playlists.iter().enumerate() {
                            let playlist_button_res = ui.add_sized(
                                [48., 48.],
                                ImageButton::new(include_image!("../../assets/icons/default.png"))
                                    .rounding(5.),
                            ).on_hover_text(&playlist.name);

                            if playlist_button_res.clicked() {
                                // set selected playlist in state
                                state.view = View::Playlist;
                            }

                            playlist_button_res.context_menu(|ui| {
                                if ui.button("Delete").clicked() {
                                    to_remove = Some(i);
                                    ui.close_menu();
                                }
                            });
                        }

                        if let Some(i) = to_remove {
                            state.config.playlists.remove(i);
                        }
                    });
                if has_playlists {
                    ui.add_space(10.);
                }

                if ui
                    .add_sized(
                        [48., 48.],
                        ImageButton::new(include_image!("../../assets/icons/add.png")).rounding(5.),
                    )
                    .on_hover_text("Create Playlist")
                    .clicked()
                {
                    state.config.playlists.insert(0, Playlist::new());
                }
            });
    }
}
