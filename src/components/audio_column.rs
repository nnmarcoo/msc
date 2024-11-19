use eframe::egui::{
    include_image, scroll_area::ScrollBarVisibility, vec2, Button, Context, ImageButton,
    ScrollArea, SidePanel,
};

use crate::msc::{State, View};

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
                            .rounding(3.),
                    )
                    .on_hover_text("Library")
                    .clicked()
                {
                    state.view = View::Library;
                }

                ui.separator();

                ScrollArea::vertical()
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                    .max_height(ui.available_height() - 64.)
                    .show(ui, |ui| {
                        for _ in 0..10 {
                            // test
                            if ui
                                .add(Button::new("").min_size(vec2(48., 48.)).rounding(3.))
                                .clicked()
                            {
                                state.view = View::Playlist;
                            }
                        }
                    });

                ui.separator();
                ui.add_sized(
                    [48., 48.],
                    ImageButton::new(include_image!("../../assets/icons/add.png")).rounding(3.),
                )
                .on_hover_text("Create Playlist");
            });
    }
}
