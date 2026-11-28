# RustProjector

A Rust reimplementation of [SoftProjector 2](https://github.com/softprojector/softprojector), a church projection application originally written in C++/Qt4.

The application interface is in Ukrainian. The project is a Cargo workspace:

| Crate | Description |
| --- | --- |
| `crates/sp-core` | GUI-independent core: data models, SQLite schema, themes, settings, schedules and import/export formats. |
| `crates/softprojector` | Desktop application built with [`eframe`/`egui`](https://github.com/emilk/egui). |

## Встановлення (Windows)

1. Відкрийте [останній реліз](https://github.com/Latte-Night-Studio/RustProjector/releases/latest) і завантажте інсталятор `...-windows-x64-setup.exe`.
2. Запустіть інсталятор. Він встановлює програму лише для поточного користувача в `%LOCALAPPDATA%\Programs\RustProjector`, тому права адміністратора не потрібні.
3. Ярлики створюються в меню «Пуск» (за бажанням — на робочому столі). База даних `spData.sqlite` зберігається в теці встановлення; під час видалення програми її можна залишити.

Портативний варіант: розпакуйте архів `...-windows-x64.zip` у будь-яку теку та запустіть `softprojector.exe`.

### Автоматичне оновлення

- Під час кожного запуску програма перевіряє останній реліз на GitHub.
- Коли доступна нова версія, унизу вікна з'являється смуга з кнопкою «Оновити». Оновлення також можна перевірити вручну на вкладці «Налаштування».
- Архів оновлення завантажується, перевіряється за SHA-256 із релізу, після чого програма перезапускається вже з новою версією.
- Для встановлених версій оновлення не потребує прав адміністратора. Якщо програму запущено з теки, доступної лише для читання, буде запропоновано підвищення прав.
- Версія 0.1.0 не містить засобу оновлення, тому перехід на новішу версію виконується один раз вручну.

> Інсталятор і програма наразі не підписані цифровим сертифікатом, тому Windows SmartScreen може показати попередження. Перевірити завантажений файл можна за допомогою файлу `.sha256` із релізу.

## Features

- **Ukrainian interface** — all user-visible labels, dialogs and messages.
- **Songs** — songbooks, categories, search, stanza-aware text formatting (verse / chorus / insert blocks), private display settings, `.sps` import and export, legacy songbook import.
- **Bible** — multiple versions, book/chapter navigation, full-text search, multi-verse selection, up to three versions on screen, `.spb` import and export.
- **Announcements** — split into slides using the original title markers, optional timing, private display settings.
- **Slideshows** — image collections stored in the database, thumbnails, reordering, JPEG preview generation.
- **Media** — image and video library. Images are rendered by the projector; videos are opened in the system default player.
- **Schedule** — a mixed service order of songs, Bible passages, announcements, slideshows and media, saved as `.spsc`.
- **Themes** — separate settings for two display outputs, Bible/song/announcement/passive sections, `.spt` import and export.
- **Projection** — a separate fullscreen-capable window on the selected monitor with keyboard control (`←`, `→`/`Space`, `B`, `Esc`).
- **Automatic updates** — GitHub Releases based, with SHA-256 verification and an in-app update banner.

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

To build the installer, download [NSIS 3.11](https://nsis.sourceforge.io/Download) (portable zip is enough) and run:

```powershell
.\installer\build-installer.ps1 -Version 0.2.1 -NsisDir C:\Tools\nsis-3.11
```

The installer and its SHA-256 file are written to `dist\`. The script accepts `-SourceExe` and `-OutputDir` for custom builds.

## Keyboard shortcuts (projector window)

| Key | Action |
| --- | --- |
| `→` / `Space` | Next slide |
| `←` | Previous slide |
| `B` | Toggle black screen |
| `Esc` | Close the projector |

## Notes

- Video playback is delegated to the operating system's default media player, because `egui` has no built-in video renderer.
- The user interface is in Ukrainian; legacy English song category names are still recognised when importing old libraries.
- Display control overlays from the original application are not reproduced.
- The updater replaces `softprojector.exe` only. Releases that add new files require running the installer once.
