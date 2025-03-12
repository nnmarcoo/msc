use egui::CentralPanel;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Msc {
    label: String,
}

impl Default for Msc {
    fn default() -> Self {
        Self {
            label: "TODO".to_owned(),
        }
    }
}

impl Msc {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

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
            ui.label(&self.label);
        });
    }
}
