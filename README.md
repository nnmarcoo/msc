<div align="center">
  <img src="assets/logo.png" width="128" alt="verse">
  <h1>verse</h1>
  <p><em>a music player built with Rust</em></p>

  ![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-0077aa?style=for-the-badge)
  ![License](https://img.shields.io/badge/license-GPL--3.0-0077aa?style=for-the-badge)
  ![Status](https://img.shields.io/badge/status-alpha-0077aa?style=for-the-badge)
</div>

---

## About

Verse is a local music player for your own files. It scans a folder, reads the
tags, and plays what it finds. No accounts, no streaming, no network.

Its interface is built from panes you arrange yourself. Rather than a fixed
window with a fixed sidebar, the layout is a tree you split, resize, lock, and
drag into whatever shape suits you, then save as a named preset. In normal mode
the chrome disappears entirely and only content remains; press `e` and the
handles, split buttons, and locks fade in over it.

> **Alpha.** The playback core and the layout system work. Several pane kinds are
> still placeholders. See [Panes](#panes) for what is and isn't implemented.

## Features

- **Playback**: play, pause, seek, volume, and loop modes (off, queue, single)
- **Library**: recursive folder scan, indexed in parallel with Rayon and cached in SQLite
- **Metadata**: ID3, Vorbis, and other tag formats via Lofty, with embedded cover art
- **Queue**: reorderable, with play-next and add-to-queue on any selection
- **Playlists**: stored in the local database
- **Ratings**: per-track, persisted
- **Search**: filters the library as you type
- **Pane layout**: split, resize, lock, drag-and-drop, and named presets
- **Artwork cache**: covers decoded once and reused for the session

## Panes

Panes are chosen per-slot from a picker in edit mode.

| Implemented | Placeholder |
| --- | --- |
| Library, Search, Queue, Controls, Timeline, Volume, Artwork, Collections | Albums, Artists, Playlists, Folders, Now Playing, History, Lyrics, Track Info, Visualiser, Equaliser, Settings |

Placeholder panes are selectable and render their name; they hold their slot in
the layout but have no content yet.

## Keys

| Key | Action |
| --- | --- |
| <kbd>Space</kbd> | Play / pause |
| <kbd>e</kbd> | Toggle edit mode |
| <kbd>1</kbd> to <kbd>9</kbd> | Switch to layout preset |

## Supported Formats

MP3, FLAC, WAV, OGG, M4A, AAC

## Install

Prebuilt binaries for Windows and Linux are on the
[releases](https://github.com/nnmarcoo/verse/releases/latest) page. Download,
extract, and run. There is no installer.

## Build

Requires Rust 1.88 or newer.

```sh
cargo build --release
cargo run -p verse-gui
```

On Linux, ALSA development headers are needed to build the audio backend:

```sh
sudo apt install libasound2-dev   # Debian/Ubuntu
sudo dnf install alsa-lib-devel   # Fedora
sudo pacman -S alsa-lib           # Arch
```

## Crates

- **core** (`verse-core`): audio backend, library scanning and database, queue, playlists, FFT analyzer
- **gui** (`verse-gui`): the [iced](https://iced.rs) frontend and pane system; builds the `verse` binary

The core is a library with no dependency on the frontend. `Player` and `Library`
are siblings rather than nested, so playback calls that need to resolve a track
take `&Library` explicitly.

## Files

| | Windows | Linux |
| --- | --- | --- |
| Config | `%APPDATA%\verse\config.toml` | `~/.config/verse/config.toml` |
| Library | `%APPDATA%\verse\data\library.db` | `~/.local/share/verse/library.db` |

## Docs

Design notes for the larger subsystems live in [docs/](docs/):
[panes.md](docs/panes.md) for the layout system,
[overlay-cursor.md](docs/overlay-cursor.md) for the iced overlay hit-testing
problem, and [layout.md](docs/layout.md) for the `pane_grid` approach it
replaced.

## Privacy

Verse is entirely local. No telemetry, no analytics, no network requests. Your
library stays on your machine.

## License

[GPL-3.0-only](LICENSE)
