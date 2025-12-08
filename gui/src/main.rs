use iced::widget::{button, column, container, horizontal_rule, row, slider, text, vertical_space};
use iced::{Element, Length, Subscription, Task, Theme};
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
            volume: 0.5,
            library_path: String::from("D:\\audio"),
            timeline: 0.0,
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
                    self.status = "Next track".into();
                }
            }
            Message::PlayPrevious => {
                if let Err(e) = self.player.play_previous() {
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
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<Message> {
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

        let top_bar = column![
            text("MSC Music Player").size(32),
            text(track_info).size(16),
            text(self.status.clone()).size(14),
        ]
        .spacing(4);

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

        // MASTER LAYOUT
        let content = column![
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
        .spacing(20);

        container(content)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
    }
}
