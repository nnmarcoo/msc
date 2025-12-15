use msc_core::Player;
use std::{
    path::Path,
    thread::{self},
    time::Duration,
};

fn main() {
    let mut player = Player::new().unwrap();

    player.populate_library(Path::new("D:\\audio"));

    thread::sleep(Duration::from_secs(60));
}
