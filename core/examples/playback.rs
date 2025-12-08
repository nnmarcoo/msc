use msc_core::Player;
use std::{path::Path, thread, time::Duration};

fn main() {
    let mut player = Player::new().expect("Failed to create player");

    println!("Loading library...");
    player.populate_library(Path::new("D:\\audio"));

    println!("Queueing library...");
    player.queue_library();

    //player.shuffle_queue();

    player.set_volume(0.1);

    println!("Starting playback...");
    if let Err(e) = player.play_next() {
        eprintln!("Error playing track: {}", e);
        return;
    }

    println!("Playing... Press Ctrl+C to stop");

    loop {
        thread::sleep(Duration::from_millis(100));

        if let Err(e) = player.update() {
            eprintln!("Error updating player: {}", e);
        }

        if player.is_playing() {
            let position = player.position();
            print!("\rPosition: {:.2}s", position);
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
    }
}
