//! Core library of the SoftProjector Rust port.
//!
//! The library is intentionally GUI independent.  It implements the original
//! SoftProjector 2 SQLite schema, data models, song/announcement stanza
//! formatting, Bible lookups, themes, settings, schedules and the import/export
//! file formats used by the Qt application.

pub mod announcement;
pub mod bible;
pub mod color;
pub mod database;
pub mod error;
pub mod font;
pub mod import_export;
pub mod media;
pub mod schedule;
pub mod settings;
pub mod slideshow;
pub mod song;
pub mod theme;

pub use announcement::{AnnounceSlide, Announcement};
pub use bible::{
    BibleBook, BibleHistory, BibleSearch, BibleSession, BibleVerse, BibleVersion, Verse,
};
pub use color::RgbColor;
pub use database::{data_dir, Database};
pub use error::{Result, SpError};
pub use font::FontSpec;
pub use media::MediaItem;
pub use schedule::{ScheduleItem, ScheduleKind};
pub use settings::{
    BibleVersionSettings, DisplayControlsSettings, GeneralSettings, Settings, SlideShowSettings,
};
pub use slideshow::{SlideShow, SlideShowInfo, SlideShowItem};
pub use song::{category_name_uk, Song, Songbook, Stanza, CATEGORIES, CATEGORIES_UK};
pub use theme::{AnnounceSettings, BibleSettings, PassiveSettings, SongSettings, Theme, ThemeInfo};

/// The database schema version understood by this port.
pub const DB_VERSION: i64 = 2;

/// Version string shown by the application.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
