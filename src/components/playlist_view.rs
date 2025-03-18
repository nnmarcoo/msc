use std::cmp::max;

use egui::{Context, Grid, ScrollArea, Ui};

use crate::core::playlist::Playlist;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct PlayListView {
    playlists: Vec<Playlist>,
    prev_size: f32,
}

impl PlayListView {
    pub fn new() -> Self {
        PlayListView { playlists: 
            vec![
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\bass.jpg".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\break.png".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\brother.jpg".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\chillaxin.png".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\drwyd.png".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\no.jpg".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\over.png".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\ppur.jpg".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\punk.png".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\vamp.jpg".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\xtreem hy.jpg".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\zooom.png".to_string()),
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\debug.jpg".to_string()),
            ],
            prev_size: 0.,
        }   
    }

    pub fn show(&mut self, ui: &mut Ui, ctx: &Context) {
        let available_width = ui.available_width();

        let zoom = ctx.zoom_factor();

        let base_gap = 3.;
        let base_min_image_size = 150.;

        let gap = base_gap * zoom;
        let min_image_size = base_min_image_size * zoom;

        let num_columns = max(
            1,
            ((available_width + gap) / (min_image_size + gap)).floor() as usize,
        );

        let image_size = (available_width - (num_columns as f32 - 1.0) * gap) / num_columns as f32;

        ScrollArea::vertical().show(ui, |ui| {
            Grid::new("playlist_grid")
            .spacing([gap, gap])
            .show(ui, |ui| { 
                for (i, playlist) in self.playlists.iter_mut().enumerate() {       
                    playlist.display_or_load(zoom, image_size, ui);
                    
                    if (i + 1) % num_columns == 0 {
                        ui.end_row();
                    }
                }
                ui.end_row();
            });
        });
        
    }
}
