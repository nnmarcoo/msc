use blake3::Hash;
use iced::widget::{
    button, column, container, horizontal_rule, image as iced_image, row, scrollable, slider, text,
    vertical_space,
};
use iced::{Color, Element, Length, Subscription, Task, Theme};
use image::{DynamicImage, GenericImageView};
use msc_core::Player;
use std::path::Path;
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application("MSC - Music Player", MusicPlayer::update, MusicPlayer::view)
        .subscription(MusicPlayer::subscription)
        .theme(|_| Theme::Dark)
        .run()
}

struct MusicPlayer {
    player: Player,
    status: String,
    volume: f32,
    library_path: String,
    timeline: f64,

    // Artwork state
    current_artwork: Option<iced::widget::image::Handle>,
    current_track_id: Option<Hash>,
    loading_artwork: bool,
}

#[derive(Debug, Clone)]
enum Message {
    PlayNext,
    PlayPrevious,
    Play,
    Pause,
    Seek(f64),
    LoadLibrary,
    ShuffleQueue,
    VolumeChanged(f32),
    Tick,
}

impl Default for MusicPlayer {
    fn default() -> Self {
        let player = Player::new().expect("Failed to initialize player");

        MusicPlayer {
            player,
            status: String::from("Ready"),
            volume: 0.1,
            library_path: String::from("D:\\audio"),
            timeline: 0.0,
            current_artwork: None,
            current_track_id: None,
            loading_artwork: false,
        }
    }
}

impl MusicPlayer {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PlayNext => {
                if let Err(e) = self.player.start_next() {
                    self.status = format!("Error: {}", e);
                } else {
                    self.status = "Next track".into();
                }
            }
            Message::PlayPrevious => {
                if let Err(e) = self.player.start_previous() {
                    self.status = format!("Error: {}", e);
                } else {
                    self.status = "Previous track".into();
                }
            }
            Message::Play => {
                self.player.play();
                self.status = "Playing".into();
            }
            Message::Pause => {
                self.player.pause();
                self.status = "Paused".into();
            }
            Message::LoadLibrary => {
                self.player.populate_library(Path::new(&self.library_path));
                self.player.queue_library();
                self.status = "Library loaded".into();
            }
            Message::ShuffleQueue => {
                self.player.shuffle_queue();
                self.status = "Queue shuffled".into();
            }
            Message::VolumeChanged(vol) => {
                self.volume = vol;
                self.player.set_volume(vol);
            }
            Message::Seek(pos) => {
                self.timeline = pos;
                self.player.seek(pos);
            }
            Message::Tick => {
                if let Err(e) = self.player.update() {
                    self.status = format!("Update error: {}", e);
                }
                self.timeline = self.player.position();

                // Check if track changed and load artwork
                if let Some(track) = self.player.current_track() {
                    if self.current_track_id != Some(track.id) {
                        self.current_track_id = Some(track.id);

                        // Clear old artwork immediately when track changes
                        self.current_artwork = None;

                        // Try to get artwork - this will return immediately if cached,
                        // or return None and start loading in background if not
                        if let Some(artwork) = self.player.art().get(&track) {
                            self.current_artwork = Some(convert_to_handle(&artwork));
                            self.loading_artwork = false;
                        } else {
                            // Not cached yet - it's now loading in background
                            self.loading_artwork = true;
                        }
                    } else if self.loading_artwork {
                        // Same track but still loading - check again
                        if let Some(artwork) = self.player.art().get(&track) {
                            self.current_artwork = Some(convert_to_handle(&artwork));
                            self.loading_artwork = false;
                        }
                    }
                } else {
                    // No track playing
                    self.current_track_id = None;
                    self.current_artwork = None;
                    self.loading_artwork = false;
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        // ALBUM ARTWORK
        let artwork_widget: Element<Message> = if let Some(handle) = &self.current_artwork {
            iced_image(handle.clone()).width(256).height(256).into()
        } else if self.loading_artwork {
            container(text("Loading...").size(14))
                .width(256)
                .height(256)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else {
            container(text("No artwork").size(14))
                .width(256)
                .height(256)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };

        // TRACK INFO
        let track_info = if let Some(track) = self.player.current_track() {
            format!(
                "Title: {}\nArtist: {}\nAlbum: {}\nGenre: {}\nDuration: {:.2} sec",
                track.metadata.title_or_default(),
                track.metadata.artist_or_default(),
                track.metadata.album_or_default(),
                track.metadata.genre_or_default(),
                track.metadata.duration()
            )
        } else {
            "No track loaded".into()
        };

        let track_info_column = column![
            text(track_info).size(16),
            text(self.status.clone()).size(14),
        ]
        .spacing(4);

        let top_bar = row![
            artwork_widget,
            column![text("MSC Music Player").size(32), track_info_column,].spacing(10)
        ]
        .spacing(20);

        // TIMELINE (seek bar)
        let pos = self.timeline;
        let timeline = column![
            text(format!("Time: {:.2}s", pos)).size(14),
            slider(0.0..=600.0, pos, Message::Seek)
                .step(0.1)
                .width(Length::Fill),
        ]
        .spacing(10);

        // PLAYBACK CONTROLS
        let controls = row![
            button("⏮ Prev").on_press(Message::PlayPrevious),
            button("▶ Play").on_press(Message::Play),
            button("⏸ Pause").on_press(Message::Pause),
            button("⏭ Next").on_press(Message::PlayNext),
        ]
        .spacing(20)
        .padding(5);

        // VOLUME
        let volume = row![
            text("🔊 Volume").size(14),
            slider(0.0..=1.0, self.volume, Message::VolumeChanged)
                .step(0.01)
                .width(Length::Fixed(200.0)),
            text(format!("{:.0}%", self.volume * 100.0)).size(14),
        ]
        .spacing(10)
        .padding([10, 0]);

        // LIBRARY
        let library_controls = row![
            button("Load Library").on_press(Message::LoadLibrary),
            button("Shuffle").on_press(Message::ShuffleQueue),
        ]
        .spacing(20);

        // QUEUE DISPLAY
        let queue_widget = self.build_queue_view();

        // MASTER LAYOUT with two columns
        let left_panel = column![
            top_bar,
            vertical_space(),
            horizontal_rule(1),
            timeline,
            vertical_space(),
            controls,
            volume,
            vertical_space(),
            horizontal_rule(1),
            library_controls,
        ]
        .padding(20)
        .spacing(20)
        .width(Length::FillPortion(2));

        let right_panel = container(queue_widget)
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .padding(20);

        let content = row![left_panel, right_panel]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
    }

    fn build_queue_view(&self) -> Element<Message> {
        let title = text("Queue").size(24);

        let mut queue_items = column![].spacing(4);

        // Add upcoming tracks
        let current_id = self.player.current_track().map(|t| t.id);

        // Get queue from player
        if let Some(tracks) = &self.player.library().tracks {
            // Current track (highlighted)
            if let Some(current) = current_id {
                if let Some(track) = tracks.get(&current) {
                    let track_name = format!(
                        "▶ {} - {}",
                        track.metadata.artist_or_default(),
                        track.metadata.title_or_default()
                    );
                    queue_items = queue_items.push(
                        container(text(track_name).size(14))
                            .style(|_theme: &Theme| container::Style {
                                background: Some(Color::from_rgb(0.2, 0.4, 0.6).into()),
                                text_color: Some(Color::WHITE),
                                ..Default::default()
                            })
                            .padding(8)
                            .width(Length::Fill),
                    );
                }
            }

            // Upcoming tracks
            for track_id in self.player.queue().upcoming() {
                if let Some(track) = tracks.get(track_id) {
                    let track_name = format!(
                        "{} - {}",
                        track.metadata.artist_or_default(),
                        track.metadata.title_or_default()
                    );
                    queue_items = queue_items.push(
                        container(text(track_name).size(14))
                            .padding(8)
                            .width(Length::Fill),
                    );
                }
            }
        }

        let queue_container = scrollable(queue_items).height(Length::Fill);

        column![title, horizontal_rule(1), queue_container]
            .spacing(10)
            .into()
    }
}

// Convert DynamicImage to iced Handle
fn convert_to_handle(img: &DynamicImage) -> iced::widget::image::Handle {
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    let pixels = rgba.into_raw();

    iced::widget::image::Handle::from_rgba(width, height, pixels)
}
