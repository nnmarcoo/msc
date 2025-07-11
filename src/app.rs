use egui::{CentralPanel, Context, Visuals};
use egui_extras::install_image_loaders;

use crate::{
    components::{
        audio_controls::AudioControls, main_panel::MainPanel, play_panel::PlayPanel,
        title_bar::TitleBar,
    },
    core::helps::add_font,
    resize::handle_resize,
    state::State,
};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Msc {
    pub state: State,
    pub titel_bar: TitleBar,
    pub audio_controls: AudioControls,
    pub play_panel: PlayPanel,
    pub main_panel: MainPanel,
}

impl Default for Msc {
    fn default() -> Self {
        Self {
            state: State::default(),
            titel_bar: TitleBar::new(),
            audio_controls: AudioControls::new(),
            play_panel: PlayPanel::new(),
            main_panel: MainPanel::new(),
        }
    }
}

impl Msc {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_image_loaders(&cc.egui_ctx);
        add_font(&cc.egui_ctx);

        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        //println!("{:#?}", cc.egui_ctx.style().visuals.clone());

        cc.egui_ctx.set_visuals(Visuals {
            //panel_fill: egui::Color32::RED,
            //override_text_color: Some(Color32::DARK_RED),
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

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.state.init();
        CentralPanel::default().show(ctx, |_ui| {
            handle_resize(self, ctx);
            self.titel_bar.show(ctx, &mut self.state);
            self.audio_controls.show(ctx, &mut self.state);
            self.play_panel.show(ctx, &mut self.state);
            self.main_panel.show(ctx, &mut self.state);
        });
    }
}
