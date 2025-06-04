use egui::{Color32, Direction, Label, Layout, RichText, Spinner, Ui};

use crate::{
    structs::{State, View},
    widgets::link_label::link_label,
};

pub fn show_loading(ui: &mut Ui) {
    ui.allocate_ui_with_layout(
        ui.available_size(),
        Layout::centered_and_justified(Direction::TopDown),
        |ui| {
            ui.add(Spinner::new().size(48.));
        },
    );
}

pub fn show_empty_library(ui: &mut Ui, state: &mut State) {
    ui.vertical(|ui| {
        ui.add_space(ui.available_height() / 2. - 20.);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2. - 60.);
            ui.add(Label::new("Audio folder empty!"));
        });
        ui.add_space(10.);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2. - 30.);

            if ui
                .add(link_label(
                    RichText::new("Settings").color(Color32::WHITE),
                    Color32::WHITE,
                ))
                .clicked()
            {
                state.view = View::Settings;
            }
        });
    });
}
