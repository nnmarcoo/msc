use eframe::{
    egui::{CentralPanel, Context, Frame as gFrame, Margin, ResizeDirection},
    App, CreationContext, Frame,
};
use egui_extras::install_image_loaders;

use crate::{
    components::{
        audio_column::show_audio_column, audio_controls::AudioControls, main_area::show_main_area,
        title_bar::show_title_bar,
    },
    util::handle_resize,
};

pub struct Msc {
    pub resizing: Option<ResizeDirection>,
    pub is_maximized: bool,
    pub is_dragging: bool,
    pub audio_controls: AudioControls,
}

impl Default for Msc {
    fn default() -> Self {
        Self {
            resizing: None,
            is_maximized: false,
            is_dragging: false,
            audio_controls: AudioControls::new(),
        }
    }
}

impl Msc {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        install_image_loaders(&cc.egui_ctx);
        Self::default()
    }
}

impl App for Msc {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default()
            .frame(
                gFrame::default()
                    .inner_margin(Margin::ZERO)
                    .fill(ctx.style().visuals.panel_fill),
            )
            .show(ctx, |ui| {
                self.is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                handle_resize(self, ctx);

                show_title_bar(self, ctx);
                self.audio_controls.show(ctx);
                show_audio_column(self, ctx);
                show_main_area(self, ctx);
            });
    }
}
