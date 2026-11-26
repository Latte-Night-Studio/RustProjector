use crate::error::{Result, SpError};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Name of the main database file used by the original application.
pub const DB_FILE: &str = "spData.sqlite";

/// Return the directory where `spData.sqlite` should live.
///
/// The original Qt application puts the database next to the executable, except
/// when it is installed into `C:\Program Files`, in which case it uses
/// `C:\ProgramData\SoftProjector`.
///
/// For development and testing the `SOFTPROJECTOR_DATA` environment variable can
/// be used to override the directory.
pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SOFTPROJECTOR_DATA") {
        let dir = PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&dir);
        return dir;
    }

    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let exe_dir = exe.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let exe_text = exe.to_string_lossy().to_lowercase();

    if exe_text.contains("program files") {
        if let Ok(program_data) = std::env::var("PROGRAMDATA") {
            let dir = PathBuf::from(program_data).join("SoftProjector");
            if std::fs::create_dir_all(&dir).is_ok() {
                return dir;
            }
        }
        if let Ok(public) = std::env::var("PUBLIC") {
            let dir = PathBuf::from(public).join("SoftProjector");
            if std::fs::create_dir_all(&dir).is_ok() {
                return dir;
            }
        }
    }

    // When launched with `cargo run` the executable lives in target/debug.
    // Prefer a database in the current working directory if one exists there,
    // otherwise keep everything next to the executable.
    let cwd_db = std::env::current_dir().ok().map(|d| d.join(DB_FILE));
    if let Some(cwd_db) = cwd_db {
        if cwd_db.exists() && !exe_dir.join(DB_FILE).exists() {
            if let Some(parent) = cwd_db.parent() {
                return parent.to_path_buf();
            }
        }
    }

    exe_dir
}

/// A connection to the main SoftProjector database.
pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    /// Open the database from the platform default location, creating it when
    /// necessary.
    pub fn open_default() -> Result<Self> {
        Self::open(data_dir().join(DB_FILE))
    }

    /// Open (or create) the database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let existed = path.exists();
        let conn = Connection::open(&path)?;
        conn.busy_timeout(std::time::Duration::from_secs(10))?;

        if !existed {
            create_schema(&conn)?;
        } else {
            let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
            if version != crate::DB_VERSION {
                return Err(SpError::DatabaseVersion {
                    found: version,
                    expected: crate::DB_VERSION,
                });
            }
            // Make sure a partially created database is still usable.
            create_schema_if_missing(&conn)?;
        }

        Ok(Self { conn, path })
    }

    /// Open a fresh in-memory database, used by tests and the demo mode.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        create_schema(&conn)?;
        Ok(Self {
            conn,
            path: PathBuf::from(":memory:"),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    pub fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>> {
        Ok(self.conn.transaction()?)
    }

    /// Read a setting blob by type, e.g. `general` or `bible1`.
    pub fn setting_blob(&self, kind: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT sets FROM Settings WHERE type = ?1 LIMIT 1")?;
        let mut rows = stmt.query([kind])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get::<_, Option<String>>(0)?.unwrap_or_default()))
        } else {
            Ok(None)
        }
    }

    /// Replace a setting blob by type.
    pub fn set_setting_blob(&self, kind: &str, value: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM Settings WHERE type = ?1", [kind])?;
        self.conn.execute(
            "INSERT INTO Settings (type, sets) VALUES (?1, ?2)",
            (kind, value),
        )?;
        Ok(())
    }
}

/// Create the complete version 2 schema of the original application.
pub fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA user_version = 2;
        CREATE TABLE IF NOT EXISTS 'Announcements' ('id' INTEGER PRIMARY KEY  AUTOINCREMENT  NOT NULL  UNIQUE , 'title' TEXT, 'text' TEXT, 'usePrivate' BOOL, 'useAuto' BOOL, 'loop' BOOL, 'slideTime' INTEGER, 'useBackground' BOOL, 'backgoundPath' TEXT, 'font' TEXT, 'color' TEXT, 'alignment' TEXT);
        CREATE TABLE IF NOT EXISTS 'BibleBooks' ('bible_id' INTEGER, 'id' INTEGER, 'book_name' TEXT, 'chapter_count' INTEGER DEFAULT 0);
        CREATE TABLE IF NOT EXISTS 'BibleVerse' ('verse_id' TEXT, 'bible_id' TEXT, 'book' TEXT, 'chapter' INTEGER, 'verse' INTEGER, 'verse_text' TEXT);
        CREATE TABLE IF NOT EXISTS 'BibleVersions' ('id' INTEGER PRIMARY KEY  AUTOINCREMENT  NOT NULL, 'bible_name' TEXT, 'abbreviation' TEXT, 'information' TEXT, 'right_to_left' INTEGER DEFAULT 0);
        CREATE TABLE IF NOT EXISTS 'Media' ('long_path' TEXT, 'short_path' TEXT);
        CREATE TABLE IF NOT EXISTS 'Settings' ('type' TEXT, 'sets' TEXT);
        CREATE TABLE IF NOT EXISTS 'SlideShows' ('id' INTEGER PRIMARY KEY  AUTOINCREMENT  NOT NULL , 'name' TEXT, 'info' TEXT);
        CREATE TABLE IF NOT EXISTS 'Slides' ('id' INTEGER PRIMARY KEY  AUTOINCREMENT  NOT NULL , 'ss_id' INTEGER, 'p_order' INTEGER, 'name' TEXT, 'path' TEXT, 'pix' BLOB, 'pix_small' BLOB, 'pix_prev' BLOB);
        CREATE TABLE IF NOT EXISTS 'Songbooks' ('id' INTEGER PRIMARY KEY  AUTOINCREMENT  NOT NULL , 'name' TEXT, 'info' TEXT);
        CREATE TABLE IF NOT EXISTS 'Songs' ('id' INTEGER PRIMARY KEY  AUTOINCREMENT  NOT NULL , 'songbook_id' INTEGER, 'number' INTEGER, 'title' TEXT, 'category' INTEGER DEFAULT 0, 'tune' TEXT, 'words' TEXT, 'music' TEXT, 'song_text' TEXT, 'notes' TEXT, 'use_private' BOOL, 'alignment_v' INTEGER, 'alignment_h' INTEGER, 'color' INTEGER, 'font' TEXT, 'info_color' INTEGER, 'info_font' TEXT, 'ending_color' INTEGER, 'ending_font' TEXT, 'use_background' BOOL, 'background_name' TEXT, 'background' BLOB, 'count' INTEGER DEFAULT 0, 'date' TEXT);
        CREATE TABLE IF NOT EXISTS 'ThemeAnnounce' ('theme_id' INTEGER, 'disp' INTEGER, 'use_shadow' BOOL, 'use_fading' BOOL, 'use_blur_shadow' BOOL, 'use_background' BOOL, 'background_name' TEXT, 'background' BLOB, 'text_font' TEXT, 'text_color' INTEGER, 'text_align_v' INTEGER, 'text_align_h' INTEGER, 'use_disp_2' BOOL);
        CREATE TABLE IF NOT EXISTS 'ThemeBible' ('theme_id' INTEGER, 'disp' INTEGER, 'use_shadow' BOOL, 'use_fading' BOOL, 'use_blur_shadow' BOOL, 'use_background' BOOL, 'background_name' TEXT, 'background' BLOB, 'text_font' TEXT, 'text_color' INTEGER, 'text_align_v' INTEGER, 'text_align_h' INTEGER, 'caption_font' TEXT, 'caption_color' INTEGER, 'caption_align' INTEGER, 'caption_position' INTEGER, 'use_abbr' BOOL, 'screen_use' INTEGER, 'screen_position' INTEGER, 'use_disp_2' BOOL);
        CREATE TABLE IF NOT EXISTS 'ThemePassive' ('theme_id' INTEGER, 'disp' INTEGER, 'use_background' BOOL, 'background_name' TEXT, 'background' BLOB, 'use_disp_2' BOOL);
        CREATE TABLE IF NOT EXISTS 'ThemeSong' ('theme_id' INTEGER, 'disp' INTEGER, 'use_shadow' BOOL, 'use_fading' BOOL, 'use_blur_shadow' BOOL, 'show_stanza_title' BOOL, 'show_key' BOOL, 'show_number' BOOL, 'info_color' INTEGER, 'info_font' TEXT, 'info_align' INTEGER, 'show_song_ending' BOOL, 'ending_color' INTEGER, 'ending_font' TEXT, 'ending_type' INTEGER, 'ending_position' INTEGER, 'use_background' BOOL, 'background_name' TEXT, 'background' BLOB, 'text_font' TEXT, 'text_color' INTEGER, 'text_align_v' INTEGER, 'text_align_h' INTEGER, 'screen_use' INTEGER, 'screen_position' INTEGER, 'use_disp_2' BOOL);
        CREATE TABLE IF NOT EXISTS 'Themes' ('id' INTEGER PRIMARY KEY  AUTOINCREMENT  NOT NULL , 'name' TEXT, 'comment' TEXT);
        CREATE INDEX IF NOT EXISTS idx_songs_songbook ON Songs(songbook_id);
        CREATE INDEX IF NOT EXISTS idx_verse_bible ON BibleVerse(bible_id, book, chapter);
        CREATE INDEX IF NOT EXISTS idx_books_bible ON BibleBooks(bible_id);
        "#,
    )?;
    Ok(())
}

fn create_schema_if_missing(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'Songs'",
        [],
        |row| row.get(0),
    )?;
    if count == 0 {
        create_schema(conn)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_schema() {
        let db = Database::open_in_memory().unwrap();
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(count >= 15);
        let version: i64 = db
            .conn()
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, crate::DB_VERSION);
    }
}
