<div align="center">
  <h1>verse</h1>
  <p><em>cross-platform music player built with Rust</em></p>

  ![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-0077aa?style=for-the-badge)
  ![Status](https://img.shields.io/badge/status-wip-0077aa?style=for-the-badge)
</div>

---

Music player with both a graphical and terminal interface, sharing a common core for playback and library management.

## Crates

- **core** — audio engine, library scanning, queue, database, FFT analyzer
- **gui** — pane-based graphical interface with OS media controls
- **tui** — terminal interface with image support

## Features

- **Playback** — play, pause, seek, volume, loop modes (none / queue / single)
- **Library** — recursive folder scan with parallel indexing via Rayon
- **Metadata** — ID3, Vorbis, and other tags via Lofty; album art caching
- **Spectrum analyzer** — real-time 32-bin FFT visualization
- **Queue** — dynamic playback queue with track management
- **Playlists** — SQLite-backed user playlists
- **Media session** — OS-level media controls (play/pause/next from taskbar, etc.)
- **GUI panes** — library, queue, artwork, collections, track info, spectrum, VU meters, settings

## Supported Formats

MP3, FLAC, WAV, OGG, M4A, AAC

## Build

```sh
cargo build --release
```

To run a specific frontend:

```sh
cargo run -p gui
cargo run -p tui
```
