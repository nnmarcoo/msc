use eframe::egui::{Context, ScrollArea, SidePanel};

use crate::msc::Msc;

pub fn show_audio_column(_app: &mut Msc, ctx: &Context) {
    SidePanel::left("audio_column")
        .resizable(false)
        .exact_width(50.)
        .show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.label("audio");
            });
        });
}
