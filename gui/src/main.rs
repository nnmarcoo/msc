use iced::widget::{button, column, container, row, slider, text};
use iced::{Element, Subscription, Task, Theme};
use msc_core::Player;
use std::path::Path;
use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        "MSC - Music Player",
        MusicPlayer::update,
        MusicPlayer::view,
    )
    .subscription(MusicPlayer::subscription)
    .theme(|_| Theme::Dark)
    .run()
}

struct MusicPlayer {
    player: Player,
    status: String,
    volume: f32,
    library_path: String,
}

#[derive(Debug, Clone)]
enum Message {
    PlayNext,
    PlayPrevious,
    Play,
    Pause,
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
            volume: 0.2,
            library_path: String::from("D:\\audio"),
        }
    }
}

impl MusicPlayer {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PlayNext => {
                if let Err(e) = self.player.play_next() {
                    self.status = format!("Error: {}", e);
                } else {
                    self.status = String::from("Playing next track");
                }
            }
            Message::PlayPrevious => {
                if let Err(e) = self.player.play_previous() {
                    self.status = format!("Error: {}", e);
                } else {
                    self.status = String::from("Playing previous track");
                }
            }
            Message::Play => {
                self.player.play();
                self.status = String::from("Resumed");
            }
            Message::Pause => {
                self.player.pause();
                self.status = String::from("Paused");
            }
            Message::LoadLibrary => {
                self.player.populate_library(Path::new(&self.library_path));
                self.player.queue_library();
                self.status = String::from("Library loaded and queued");
            }
            Message::ShuffleQueue => {
                self.player.shuffle_queue();
                self.status = String::from("Queue shuffled");
            }
            Message::VolumeChanged(vol) => {
                self.volume = vol;
                self.player.set_volume(vol);
                self.status = format!("Volume: {:.0}%", vol * 100.0);
            }
            Message::Tick => {
                if let Err(e) = self.player.update() {
                    self.status = format!("Update error: {}", e);
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        let position_text = if self.player.is_playing() {
            format!("Position: {:.2}s", self.player.position())
        } else {
            String::from("Not playing")
        };

        let track_info = if let Some(track_id) = self.player.current_track_id() {
            format!("Track ID: {}...", &track_id.to_hex()[..16])
        } else {
            String::from("No track loaded")
        };

        let playback_controls = row![
            button("Previous").on_press(Message::PlayPrevious),
            button("Play").on_press(Message::Play),
            button("Pause").on_press(Message::Pause),
            button("Next").on_press(Message::PlayNext),
        ]
        .spacing(10);

        let library_controls = row![
            button("Load Library").on_press(Message::LoadLibrary),
            button("Shuffle Queue").on_press(Message::ShuffleQueue),
        ]
        .spacing(10);

        let volume_control = row![
            text("Volume:").size(14),
            slider(0.0..=1.0, self.volume, Message::VolumeChanged)
                .step(0.01),
            text(format!("{:.0}%", self.volume * 100.0)).size(14),
        ]
        .spacing(10);

        let content = column![
            text("MSC Music Player").size(32),
            text(self.status.clone()).size(16),
            text(track_info).size(12),
            text(position_text).size(14),
            volume_control,
            playback_controls,
            library_controls,
        ]
        .spacing(20)
        .padding(20);

        container(content).center(iced::Length::Fill).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
    }
}
