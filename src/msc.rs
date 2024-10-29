use eframe::{
    egui::{CentralPanel, Context},
    App, CreationContext, Frame,
};

pub struct Msc {}

impl Default for Msc {
    fn default() -> Self {
        Self {}
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
            ui.label("Hello, world!");
        });
    }
}
