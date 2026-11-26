# RustProjector

A Rust reimplementation of [SoftProjector 2](https://github.com/softprojector/softprojector), a church projection application originally written in C++/Qt4.

The project is a Cargo workspace:

| Crate | Description |
| --- | --- |
| `crates/sp-core` | GUI-independent core: data models, SQLite schema, themes, settings, schedules and import/export formats. |
| `crates/softprojector` | Desktop application built with [`eframe`/`egui`](https://github.com/emilk/egui). |

## Features

- **Songs** — songbooks, categories, search, stanza-aware text formatting (verse / chorus / insert blocks), private display settings, `.sps` import and export, legacy songbook import.
- **Bible** — multiple versions, book/chapter navigation, full-text search, multi-verse selection, up to three versions on screen, `.spb` import and export.
- **Announcements** — split into slides using the original title markers, optional timing, private display settings.
- **Slideshows** — image collections stored in the database, thumbnails, reordering, JPEG preview generation.
- **Media** — image and video library. Images are rendered by the projector; videos are opened in the system default player.
- **Schedule** — a mixed service order of songs, Bible passages, announcements, slideshows and media, saved as `.spsc`.
- **Themes** — separate settings for two display outputs, Bible/song/announcement/passive sections, `.spt` import and export.
- **Projection** — a separate fullscreen-capable window on the selected monitor with keyboard control (`←`, `→`/`Space`, `B`, `Esc`).

## Database compatibility

The port understands the SoftProjector 2 schema (`user_version = 2`) and uses the same `spData.sqlite` file name. The database is stored next to the executable, or in `%PROGRAMDATA%\SoftProjector` when running from `Program Files`, matching the original application. Set the `SOFTPROJECTOR_DATA` environment variable to use a custom directory:

```powershell
$env:SOFTPROJECTOR_DATA = "D:\SoftProjectorData"
```

## Building

Install the Rust toolchain (MSVC on Windows) and run:

```powershell
cargo build --release
cargo test --workspace
```

The application binary is written to `target/release/softprojector.exe`.

## Keyboard shortcuts (projector window)

| Key | Action |
| --- | --- |
| `→` / `Space` | Next slide |
| `←` | Previous slide |
| `B` | Toggle black screen |
| `Esc` | Close the projector |

## Notes

- Video playback is delegated to the operating system's default media player, because `egui` has no built-in video renderer.
- The Qt translation files are not used; the user interface is currently in English.
- Display control overlays from the original application are not reproduced.
