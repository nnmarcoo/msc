#[derive(serde::Deserialize, serde::Serialize)]
pub struct MainPanel {}

impl MainPanel {
  pub fn new() -> Self {
    MainPanel {}
  }
  pub fn show(&mut self, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
      ui.label("Main panel");
    });
  }
}