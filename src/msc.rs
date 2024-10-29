use eframe::{
    egui::{CentralPanel, Context},
    App, CreationContext, Frame,
};

use crate::components::title_bar::show_title_bar;

pub struct Msc {
}

impl Default for Msc {
    fn default() -> Self {
        Self {
        }
    }
}

impl Msc {
    pub fn new(_cc: &CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl App for Msc {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            show_title_bar(ctx);
        });
    }
}
