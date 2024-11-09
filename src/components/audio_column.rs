use eframe::egui::{
    include_image, scroll_area::ScrollBarVisibility, vec2, Button, Context, ImageButton,
    ScrollArea, SidePanel,
};

pub struct AudioColumn {}

impl AudioColumn {
    pub fn new() -> Self {
        AudioColumn {}
    }

    pub fn show(&mut self, ctx: &Context) {
        SidePanel::left("audio_column")
            .resizable(false)
            .exact_width(60.)
            .show(ctx, |ui| {
                ui.add(Button::new("").min_size(vec2(50., 50.)).rounding(3.));
                ui.separator();

                ScrollArea::vertical()
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                    .max_height(ui.available_height() - 64.)
                    .show(ui, |ui| {
                        for _ in 0..10 {
                            // test
                            ui.add(Button::new("").min_size(vec2(50., 50.)).rounding(3.));
                        }
                    });

                ui.separator();
                ui.add_sized(
                    [50., 50.],
                    ImageButton::new(include_image!("../../assets/icons/add.png")).rounding(3.),
                )
                .on_hover_text("Create Playlist");
            });
    }
}
