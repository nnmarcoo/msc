use egui::Ui;
use crate::core::Playlist::Playlist;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct PlayListView {
    playlists: Vec<Playlist>,
}

impl PlayListView {
    pub fn new() -> Self {
        PlayListView { playlists: 
            vec![
            Playlist::new("Playlist 1".to_string(), "Description 1".to_string(), "D:\\spotify\\brother.jpg".to_string()),
        ] }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        // Get the total available width in the current UI context
        let available_width = ui.available_width();

        // Define a gap between images (in pixels)
        let gap = 10.0;

        // Define a minimum desired image size (width/height in pixels)
        let min_image_size = 100.0;

        // Calculate the maximum number of columns that can fit using the minimum image size
        let mut num_columns = ((available_width + gap) / (min_image_size + gap)).floor() as usize;
        if num_columns < 1 {
            num_columns = 1;
        }

        // Recalculate the image size so that images and gaps exactly fit the available width
        let image_size = (available_width - (num_columns as f32 - 1.0) * gap) / num_columns as f32;

        // Use egui's Grid widget to display images in a grid with spacing between them
        egui::Grid::new("playlist_grid")
            .spacing([gap, gap])
            .show(ui, |ui| {
                for (i, playlist) in self.playlists.iter().enumerate() {
                    // Display the playlist image with the calculated dimensions
                    playlist.display_or_load(image_size as u32, image_size as u32, ui);
                    
                    // End the row after num_columns images have been added
                    if (i + 1) % num_columns == 0 {
                        ui.end_row();
                    }
                }
                // Ensure the final row is ended even if it's not full
                ui.end_row();
            });
    }
}
