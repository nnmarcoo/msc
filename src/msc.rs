use eframe::{
    egui::{CentralPanel, Context, Frame as gFrame, Margin, ResizeDirection},
    App, CreationContext, Frame,
};
use egui_extras::install_image_loaders;

use crate::{
    backend::resize::handle_resize,
    components::{
        audio_column::AudioColumn, audio_controls::AudioControls, main_area::show_main_area,
        title_bar::TitleBar,
    },
};

pub struct Msc {
    pub resizing: Option<ResizeDirection>,
    pub audio_column: AudioColumn,
    pub audio_controls: AudioControls,
    pub title_bar: TitleBar,
}

impl Default for Msc {
    fn default() -> Self {
        Self {
            resizing: None,
            audio_column: AudioColumn::new(),
            audio_controls: AudioControls::new(),
            title_bar: TitleBar::new(),
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
            .show(ctx, |_ui| {
                handle_resize(self, ctx);

                self.title_bar.show(ctx);
                self.audio_controls.show(ctx);
                self.audio_column.show(ctx);
                show_main_area(self, ctx);
            });
    }
}
