use std::collections::HashMap;

use eframe::{
    egui::{CentralPanel, Context, Frame as gFrame, Margin, ResizeDirection},
    App, CreationContext, Frame,
};
use egui_extras::install_image_loaders;

use crate::{
    backend::{cfg::Config, playlist::Playlist, queue::Queue, resize::handle_resize, track::Track},
    components::{
        audio_column::AudioColumn, audio_controls::AudioControls, main_area::MainArea,
        title_bar::TitleBar,
    },
};

pub enum View {
    Playlist,
    Settings,
    _Search, // unused
    Library,
}

pub struct State {
    pub config: Config,
    pub view: View,
    pub library: HashMap<String, Track>,
    pub query: String,
    pub selected_playlist: usize,
    pub queue: Queue,
}

pub struct Msc {
    pub state: State,
    pub resizing: Option<ResizeDirection>,
    pub audio_column: AudioColumn,
    pub audio_controls: AudioControls,
    pub title_bar: TitleBar,
    pub main_area: MainArea,
}

impl Msc {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        install_image_loaders(&cc.egui_ctx);

        let config = Config::get();
        let test: Playlist = Playlist::from_directory(&config.audio_directory);

        let state = State {
            config: Config::get(),
            view: View::Library,
            library: test,
            query: String::new(),
            selected_playlist: 0,
            queue: Queue::from_playlist(test),
        };

        Self {
            state,
            resizing: None,
            audio_column: AudioColumn::new(),
            audio_controls: AudioControls::new(),
            title_bar: TitleBar::new(),
            main_area: MainArea::new(),
        }
    }
}

impl App for Msc {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default()
            .frame(
                gFrame::default()
                    .inner_margin(Margin::ZERO)
                    .fill(ctx.style().visuals.panel_fill),
            )
            .show(ctx, |_ui| {
                handle_resize(self, ctx);

                self.title_bar.show(ctx, &mut self.state);
                self.audio_controls.show(ctx, &mut self.state);
                self.audio_column.show(ctx, &mut self.state);
                self.main_area.show(ctx, &mut self.state);
            });
    }
}
