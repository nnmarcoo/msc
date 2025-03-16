use egui::{CentralPanel, Visuals};
use egui_extras::install_image_loaders;

use crate::{components::title_bar::TitleBar, resize::handle_resize, structs::WindowState};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Msc {
    pub window_state: WindowState,
    pub titel_bar: TitleBar,
}

impl Default for Msc {
    fn default() -> Self {
        Self {
            window_state: WindowState::default(),
            titel_bar: TitleBar::new(),
        }
    }
}

impl Msc {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_image_loaders(&cc.egui_ctx);
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        println!("{:#?}", cc.egui_ctx.style().visuals.clone());

        cc.egui_ctx.set_visuals(Visuals {
            //panel_fill: egui::Color32::RED,
            ..Default::default()
        });

        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }
}

impl eframe::App for Msc {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            handle_resize(self, ctx);
            self.titel_bar.show(ctx);
        });
    }
}
