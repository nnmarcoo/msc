use eframe::{
    egui::{CentralPanel, Context, ResizeDirection},
    App, CreationContext, Frame,
};
use egui_extras::install_image_loaders;

use crate::{components::title_bar::show_title_bar, util::handle_resize};

pub struct Msc {
    pub resizing: Option<ResizeDirection>,
    pub is_maximized: bool,
    pub is_dragging: bool,
}

impl Default for Msc {
    fn default() -> Self {
        Self {
            resizing: None,
            is_maximized: false,
            is_dragging: false,
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
        CentralPanel::default().show(ctx, |ui| {
            self.is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false)); // duplcate
            handle_resize(self, ctx);

            show_title_bar(self, ctx);
        });
    }
}
