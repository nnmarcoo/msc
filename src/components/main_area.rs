use eframe::egui::{CentralPanel, Context};

use crate::msc::Msc;

pub fn show_main_area(_app: &mut Msc, ctx: &Context) {
  CentralPanel::default().show(ctx, |ui| {
    ui.label("main");
});
}
