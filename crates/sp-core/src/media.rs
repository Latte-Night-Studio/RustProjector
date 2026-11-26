use crate::database::Database;
use crate::error::Result;
use rusqlite::params;
use std::path::Path;

/// A video entry of the `Media` table.
#[derive(Clone, Debug, Default)]
pub struct MediaItem {
    pub long_path: String,
    pub short_path: String,
    /// Aspect ratio in percent (100 = 1:1), used by the original schedule.
    pub aspect_ratio: i32,
}

impl MediaItem {
    pub fn name(&self) -> &str {
        &self.short_path
    }

    pub fn exists(&self) -> bool {
        Path::new(&self.long_path).exists()
    }

    pub fn is_video(&self) -> bool {
        let lower = self.long_path.to_lowercase();
        VIDEO_EXTENSIONS
            .iter()
            .any(|extension| lower.ends_with(extension))
    }

    pub fn load_all(db: &Database) -> Result<Vec<MediaItem>> {
        let conn = db.conn();
        let mut stmt =
            conn.prepare("SELECT long_path, short_path FROM Media ORDER BY short_path")?;
        let rows = stmt.query_map([], |row| {
            Ok(MediaItem {
                long_path: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                short_path: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                aspect_ratio: 100,
            })
        })?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub fn add(db: &Database, path: &str) -> Result<()> {
        let short = Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        db.conn().execute(
            "INSERT INTO Media (long_path, short_path) VALUES (?1, ?2)",
            params![path, short],
        )?;
        Ok(())
    }

    pub fn delete(db: &Database, short_path: &str) -> Result<()> {
        db.conn().execute(
            "DELETE FROM Media WHERE short_path = ?1",
            params![short_path],
        )?;
        Ok(())
    }

    pub fn delete_path(db: &Database, long_path: &str) -> Result<()> {
        db.conn()
            .execute("DELETE FROM Media WHERE long_path = ?1", params![long_path])?;
        Ok(())
    }
}

pub const VIDEO_EXTENSIONS: &[&str] = &[
    ".mp4", ".m4v", ".avi", ".mkv", ".wmv", ".mov", ".mpg", ".mpeg", ".webm", ".flv", ".ts",
    ".m2ts", ".ogv",
];

pub const IMAGE_EXTENSIONS: &[&str] = &[
    ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".webp", ".tif", ".tiff", ".ico", ".svg",
];

pub fn is_image_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    IMAGE_EXTENSIONS
        .iter()
        .any(|extension| lower.ends_with(extension))
}

pub fn is_video_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    VIDEO_EXTENSIONS
        .iter()
        .any(|extension| lower.ends_with(extension))
}
