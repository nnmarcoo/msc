<div align="center">
  <a href="https://github.com/nnmarcoo/verse/releases/latest"><img src="assets/logo.png" width="120" alt="verse"></a>
  <br><br>
  <h1>verse</h1>
  <p><em>a music player built with Rust</em></p>

  ![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-000000?style=for-the-badge)
  ![License](https://img.shields.io/badge/license-GPL--3.0-000000?style=for-the-badge)
  ![Status](https://img.shields.io/badge/status-alpha-000000?style=for-the-badge)
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

## Supported Formats

<table align="center">
  <thead>
    <tr><th>Format</th><th>Extension</th></tr>
  </thead>
  <tbody>
    <tr><td>AAC</td><td><code>.aac</code></td></tr>
    <tr><td>ALAC / AAC (MP4)</td><td><code>.m4a</code></td></tr>
    <tr><td>FLAC</td><td><code>.flac</code></td></tr>
    <tr><td>MP3</td><td><code>.mp3</code></td></tr>
    <tr><td>Vorbis</td><td><code>.ogg</code></td></tr>
    <tr><td>WAV</td><td><code>.wav</code></td></tr>
  </tbody>
</table>

## Panes

Panes are chosen per-slot from a picker in edit mode. Placeholder panes are
selectable and render their name; they hold their slot in the layout but have no
content yet.

<table align="center">
  <thead>
    <tr><th>Implemented</th><th>Placeholder</th></tr>
  </thead>
  <tbody>
    <tr>
      <td>Library, Search, Queue, Controls, Timeline, Volume, Artwork, Collections</td>
      <td>Albums, Artists, Playlists, Folders, Now Playing, History, Lyrics, Track Info, Visualizer, Equalizer, Settings</td>
    </tr>
  </tbody>
</table>

## Keys

Every key below is rebindable in Preferences → Keybindings. These are the
defaults.

<table align="center">
  <thead>
    <tr><th>Key</th><th>Action</th></tr>
  </thead>
  <tbody>
    <tr><td><kbd>Space</kbd></td><td>Play / pause</td></tr>
    <tr><td><kbd>←</kbd> / <kbd>→</kbd></td><td>Previous / next track</td></tr>
    <tr><td><kbd>m</kbd></td><td>Mute</td></tr>
    <tr><td><kbd>r</kbd></td><td>Cycle loop mode</td></tr>
    <tr><td><kbd>h</kbd></td><td>Shuffle queue</td></tr>
    <tr><td><kbd>e</kbd></td><td>Toggle edit mode</td></tr>
    <tr><td><kbd>s</kbd></td><td>Toggle preferences</td></tr>
    <tr><td><kbd>1</kbd> to <kbd>9</kbd>, <kbd>0</kbd></td><td>Switch to layout preset (ten slots)</td></tr>
  </tbody>
</table>

## Download

Prebuilt binaries are on the
[releases](https://github.com/nnmarcoo/verse/releases/latest) page.

| Download | Platform |
| --- | --- |
| `verse-windows-x86_64.zip` | Windows 10 or newer |
| `verse-linux-x86_64.tar.gz` | Linux (glibc, ALSA) |

Extract and run. There is no installer.

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

The workspace is two crates. `verse-core` holds the audio backend, library
scanning and database, queue, and playlists, and depends on nothing in the
frontend. `verse-gui` is the [iced](https://iced.rs) interface and pane system,
and builds the `verse` binary.

Design notes for the larger subsystems live in [docs/](docs/):
[panes.md](docs/panes.md) for the layout system,
[overlay-cursor.md](docs/overlay-cursor.md) for the iced overlay hit-testing
problem, and [layout.md](docs/layout.md) for the `pane_grid` approach it
replaced.

## Files

<table align="center">
  <thead>
    <tr><th></th><th>Windows</th><th>Linux</th></tr>
  </thead>
  <tbody>
    <tr><td>Config</td><td><code>%APPDATA%\verse\config.toml</code></td><td><code>~/.config/verse/config.toml</code></td></tr>
    <tr><td>Library</td><td><code>%APPDATA%\verse\data\library.db</code></td><td><code>~/.local/share/verse/library.db</code></td></tr>
  </tbody>
</table>

## Privacy

Verse is entirely local. No telemetry, no analytics, no network requests. Your
library stays on your machine.
