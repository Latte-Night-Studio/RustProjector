use crate::database::Database;
use crate::error::{Result, SpError};
use image::codecs::jpeg::JpegEncoder;
use image::DynamicImage;
use rusqlite::{params, OptionalExtension};
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct SlideShowInfo {
    pub id: i64,
    pub name: String,
    pub info: String,
}

#[derive(Clone, Debug, Default)]
pub struct SlideShowItem {
    pub slide_id: i64,
    pub order: i32,
    pub name: String,
    pub path: String,
    pub image: Option<Vec<u8>>,
    pub image_small: Option<Vec<u8>>,
    pub image_preview: Option<Vec<u8>>,
}

impl SlideShowItem {
    pub fn display_bytes(&self) -> Option<&[u8]> {
        if let Some(bytes) = self.image.as_ref().filter(|b| !b.is_empty()) {
            return Some(bytes);
        }
        self.image_preview.as_deref().filter(|b| !b.is_empty())
    }
}

#[derive(Clone, Debug, Default)]
pub struct SlideShow {
    pub id: i64,
    pub name: String,
    pub info: String,
    pub slides: Vec<SlideShowItem>,
}

impl SlideShow {
    pub fn load_all(db: &Database) -> Result<Vec<SlideShowInfo>> {
        let conn = db.conn();
        let mut stmt = conn.prepare("SELECT id, name, info FROM SlideShows ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(SlideShowInfo {
                id: row.get(0)?,
                name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                info: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            })
        })?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub fn load(db: &Database, id: i64) -> Result<Self> {
        let conn = db.conn();
        let mut show = SlideShow {
            id,
            ..Default::default()
        };
        let info = conn
            .query_row(
                "SELECT name, info FROM SlideShows WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                        row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    ))
                },
            )
            .optional()?;
        let Some((name, info)) = info else {
            return Err(SpError::InvalidData(format!(
                "slideshow {} does not exist",
                id
            )));
        };
        show.name = name;
        show.info = info;

        let mut stmt = conn.prepare(
            "SELECT id, p_order, name, path, pix, pix_small, pix_prev \
             FROM Slides WHERE ss_id = ?1 ORDER BY p_order, id",
        )?;
        let rows = stmt.query_map(params![id], |row| {
            Ok(SlideShowItem {
                slide_id: row.get(0)?,
                order: row.get::<_, Option<i64>>(1)?.unwrap_or(0) as i32,
                name: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                path: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                image: row.get::<_, Option<Vec<u8>>>(4)?,
                image_small: row.get::<_, Option<Vec<u8>>>(5)?,
                image_preview: row.get::<_, Option<Vec<u8>>>(6)?,
            })
        })?;
        for row in rows {
            show.slides.push(row?);
        }
        show.slides.sort_by_key(|slide| slide.order);
        Ok(show)
    }

    /// Create a new slideshow from a list of image files.
    pub fn create_from_files(
        db: &Database,
        name: &str,
        info: &str,
        paths: &[String],
    ) -> Result<Self> {
        let mut show = SlideShow {
            id: 0,
            name: name.to_string(),
            info: info.to_string(),
            slides: Vec::new(),
        };
        for (index, path) in paths.iter().enumerate() {
            let item = make_slide_item(path, index as i32)?;
            show.slides.push(item);
        }
        show.save(db)?;
        Ok(show)
    }

    pub fn add_file(&mut self, db: &Database, path: &str) -> Result<()> {
        let item = make_slide_item(path, self.slides.len() as i32)?;
        self.slides.push(item);
        self.save(db)
    }

    pub fn save(&mut self, db: &Database) -> Result<()> {
        let conn = db.conn();
        if self.id <= 0 {
            conn.execute(
                "INSERT INTO SlideShows (name, info) VALUES (?1, ?2)",
                params![self.name, self.info],
            )?;
            self.id = conn.last_insert_rowid();
        } else {
            conn.execute(
                "UPDATE SlideShows SET name = ?1, info = ?2 WHERE id = ?3",
                params![self.name, self.info, self.id],
            )?;
        }

        for (index, slide) in self.slides.iter_mut().enumerate() {
            slide.order = index as i32;
            if slide.slide_id > 0 {
                conn.execute(
                    "UPDATE Slides SET p_order = ?1, name = ?2, path = ?3, pix = ?4, pix_small = ?5, \
                     pix_prev = ?6 WHERE id = ?7",
                    params![
                        slide.order,
                        slide.name,
                        slide.path,
                        slide.image,
                        slide.image_small,
                        slide.image_preview,
                        slide.slide_id
                    ],
                )?;
            } else {
                conn.execute(
                    "INSERT INTO Slides (ss_id, p_order, name, path, pix, pix_small, pix_prev) \
                     VALUES (?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        self.id,
                        slide.order,
                        slide.name,
                        slide.path,
                        slide.image,
                        slide.image_small,
                        slide.image_preview
                    ],
                )?;
                slide.slide_id = conn.last_insert_rowid();
            }
        }

        // Remove slides that are no longer part of the show.
        let keep: Vec<i64> = self
            .slides
            .iter()
            .map(|slide| slide.slide_id)
            .filter(|id| *id > 0)
            .collect();
        let mut stmt = conn.prepare("SELECT id FROM Slides WHERE ss_id = ?1")?;
        let rows = stmt.query_map(params![self.id], |row| row.get::<_, i64>(0))?;
        let mut remove = Vec::new();
        for row in rows {
            let id = row?;
            if !keep.contains(&id) {
                remove.push(id);
            }
        }
        drop(stmt);
        for id in remove {
            conn.execute("DELETE FROM Slides WHERE id = ?1", params![id])?;
        }
        Ok(())
    }

    pub fn move_slide(&mut self, from: usize, to: usize) {
        if from < self.slides.len() && to < self.slides.len() && from != to {
            let item = self.slides.remove(from);
            self.slides.insert(to, item);
        }
    }

    pub fn delete(db: &Database, id: i64) -> Result<()> {
        db.conn()
            .execute("DELETE FROM Slides WHERE ss_id = ?1", params![id])?;
        db.conn()
            .execute("DELETE FROM SlideShows WHERE id = ?1", params![id])?;
        Ok(())
    }
}

/// Decode an image file and produce the three JPEG representations stored in
/// the original database (full, small and preview).
pub fn make_slide_item(path: &str, order: i32) -> Result<SlideShowItem> {
    let image = image::open(path).map_err(SpError::Image)?;
    let name = Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    Ok(SlideShowItem {
        slide_id: -1,
        order,
        name,
        path: path.to_string(),
        image: Some(encode_jpeg(&image, 90)?),
        image_small: Some(encode_jpeg(&image.thumbnail(200, 150), 80)?),
        image_preview: Some(encode_jpeg(&image.thumbnail(800, 600), 85)?),
    })
}

pub fn encode_jpeg(image: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let rgb = image.to_rgb8();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, quality);
    encoder
        .encode_image(&DynamicImage::ImageRgb8(rgb))
        .map_err(SpError::Image)?;
    Ok(bytes)
}

/// Load image bytes for a slide.  The original file path is preferred so that
/// newly added images keep their full quality, otherwise the stored JPEG is
/// used.
pub fn load_slide_image(slide: &SlideShowItem) -> Option<Vec<u8>> {
    if !slide.path.is_empty() {
        if let Ok(bytes) = std::fs::read(&slide.path) {
            return Some(bytes);
        }
    }
    slide.image.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slideshow_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let mut show = SlideShow {
            id: 0,
            name: "Test".into(),
            info: "Info".into(),
            slides: vec![SlideShowItem {
                slide_id: -1,
                order: 0,
                name: "a.jpg".into(),
                path: "a.jpg".into(),
                image: Some(vec![1, 2, 3]),
                image_small: None,
                image_preview: None,
            }],
        };
        show.save(&db).unwrap();
        let loaded = SlideShow::load(&db, show.id).unwrap();
        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.slides.len(), 1);
        assert_eq!(loaded.slides[0].image, Some(vec![1, 2, 3]));
    }
}
