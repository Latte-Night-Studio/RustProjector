use crate::color::RgbColor;
use crate::database::Database;
use crate::error::Result;
use crate::font::FontSpec;
use rusqlite::params;

/// Return true when a line is an announcement slide title.
pub fn is_announce_title(line: &str) -> bool {
    [
        "Announce",
        "Slide",
        "Объявление",
        "Слайд",
        "Оголошення",
        "Ankündigung",
        "Oznámení",
        "Snímek",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

/// One announcement prepared for projection.
#[derive(Clone, Debug, Default)]
pub struct AnnounceSlide {
    pub text: String,
    pub alignment_v: i32,
    pub alignment_h: i32,
    pub background_path: String,
    pub color: RgbColor,
    pub font: FontSpec,
    pub use_private_settings: bool,
}

/// An announcement row from the `Announcements` table.
#[derive(Clone, Debug)]
pub struct Announcement {
    pub id: i64,
    pub title: String,
    pub text: String,
    pub use_private_settings: bool,
    pub use_auto_next: bool,
    pub loop_slides: bool,
    pub slide_timer: i32,
    pub use_background: bool,
    pub background_path: String,
    pub font: FontSpec,
    pub color: RgbColor,
    pub alignment_v: i32,
    pub alignment_h: i32,
}

impl Default for Announcement {
    fn default() -> Self {
        Self {
            id: 0,
            title: String::new(),
            text: String::new(),
            use_private_settings: false,
            use_auto_next: true,
            loop_slides: true,
            slide_timer: 10,
            use_background: false,
            background_path: String::new(),
            font: FontSpec::default(),
            color: RgbColor::WHITE,
            alignment_v: 0,
            alignment_h: 1,
        }
    }
}

impl Announcement {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn load_all(db: &Database) -> Result<Vec<Announcement>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, title, text, usePrivate, useAuto, loop, slideTime, useBackground, \
                    backgoundPath, font, color, alignment FROM Announcements ORDER BY id",
        )?;
        let rows = stmt.query_map([], announcement_from_row)?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub fn load(db: &Database, id: i64) -> Result<Option<Announcement>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, title, text, usePrivate, useAuto, loop, slideTime, useBackground, \
                    backgoundPath, font, color, alignment FROM Announcements WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(announcement_from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn save(&mut self, db: &Database) -> Result<()> {
        if self.id > 0 {
            self.save_update(db)
        } else {
            self.save_new(db)
        }
    }

    pub fn save_new(&mut self, db: &Database) -> Result<()> {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO Announcements (title, text, usePrivate, useAuto, loop, slideTime, \
             useBackground, backgoundPath, font, color, alignment) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                self.title,
                self.text,
                self.use_private_settings,
                self.use_auto_next,
                self.loop_slides,
                self.slide_timer,
                self.use_background,
                self.background_path,
                self.font.to_qt(),
                self.color.to_qt().to_string(),
                format!("{},{}", self.alignment_v, self.alignment_h)
            ],
        )?;
        self.id = conn.last_insert_rowid();
        Ok(())
    }

    pub fn save_update(&self, db: &Database) -> Result<()> {
        let conn = db.conn();
        conn.execute(
            "UPDATE Announcements SET title = ?1, text = ?2, usePrivate = ?3, useAuto = ?4, loop = ?5, \
             slideTime = ?6, useBackground = ?7, backgoundPath = ?8, font = ?9, color = ?10, alignment = ?11 \
             WHERE id = ?12",
            params![
                self.title,
                self.text,
                self.use_private_settings,
                self.use_auto_next,
                self.loop_slides,
                self.slide_timer,
                self.use_background,
                self.background_path,
                self.font.to_qt(),
                self.color.to_qt().to_string(),
                format!("{},{}", self.alignment_v, self.alignment_h),
                self.id
            ],
        )?;
        Ok(())
    }

    pub fn delete(db: &Database, id: i64) -> Result<()> {
        db.conn()
            .execute("DELETE FROM Announcements WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Split the announcement text into slides.  Slide title lines are kept in
    /// the returned strings, exactly like the original implementation.
    pub fn announce_list(&self) -> Vec<String> {
        let mut list = Vec::new();
        let mut part = String::new();
        for line in self.text.replace("\r\n", "\n").split('\n') {
            if is_announce_title(line) {
                if !part.trim().is_empty() {
                    list.push(part.trim().to_string());
                }
                part.clear();
            }
            part.push_str(line);
            part.push('\n');
        }
        if !part.trim().is_empty() {
            list.push(part.trim().to_string());
        }
        list
    }

    /// Prepare the slide at `current` for projection.
    pub fn announce_slide(&self, current: usize) -> AnnounceSlide {
        let list = self.announce_list();
        let Some(entry) = list.get(current) else {
            return AnnounceSlide::default();
        };
        let mut lines: Vec<&str> = entry.split('\n').collect();
        if lines.first().is_some_and(|line| is_announce_title(line)) {
            lines.remove(0);
        }
        AnnounceSlide {
            text: lines.join("\n").trim().to_string(),
            alignment_v: self.alignment_v,
            alignment_h: self.alignment_h,
            background_path: self.background_path.clone(),
            color: self.color,
            font: self.font.clone(),
            use_private_settings: self.use_private_settings,
        }
    }

    pub fn slide_count(&self) -> usize {
        self.announce_list().len()
    }
}

fn announcement_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Announcement> {
    let color_text: String = row.get::<_, Option<String>>(10)?.unwrap_or_default();
    let color = color_text
        .trim()
        .parse::<u32>()
        .map(RgbColor::from_qt)
        .unwrap_or(RgbColor::WHITE);
    let alignment: String = row.get::<_, Option<String>>(11)?.unwrap_or_default();
    let mut parts = alignment.split(',');
    let alignment_v = parts
        .next()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    let alignment_h = parts
        .next()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(1);
    let font_text: String = row.get::<_, Option<String>>(9)?.unwrap_or_default();

    Ok(Announcement {
        id: row.get(0)?,
        title: row
            .get::<_, Option<String>>(1)?
            .unwrap_or_default()
            .trim()
            .to_string(),
        text: row
            .get::<_, Option<String>>(2)?
            .unwrap_or_default()
            .trim()
            .to_string(),
        use_private_settings: row.get::<_, Option<i64>>(3)?.unwrap_or(0) != 0,
        use_auto_next: row.get::<_, Option<i64>>(4)?.unwrap_or(1) != 0,
        loop_slides: row.get::<_, Option<i64>>(5)?.unwrap_or(1) != 0,
        slide_timer: row.get::<_, Option<i64>>(6)?.unwrap_or(10) as i32,
        use_background: row.get::<_, Option<i64>>(7)?.unwrap_or(0) != 0,
        background_path: row
            .get::<_, Option<String>>(8)?
            .unwrap_or_default()
            .trim()
            .to_string(),
        font: if font_text.trim().is_empty() {
            FontSpec::default()
        } else {
            FontSpec::from_qt(&font_text)
        },
        color,
        alignment_v,
        alignment_h,
    })
}

/// Load a single announcement, returning `None` when it was deleted.
pub fn load_optional(db: &Database, id: i64) -> Result<Option<Announcement>> {
    Announcement::load(db, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_slides() {
        let mut a = Announcement::default();
        a.text = "Announce 1\nFirst slide\n\nSlide 2\nSecond slide".into();
        let list = a.announce_list();
        assert_eq!(list.len(), 2);
        assert_eq!(a.announce_slide(1).text, "Second slide");
    }

    #[test]
    fn db_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let mut a = Announcement::default();
        a.title = "Welcome".into();
        a.text = "Announce\nHello".into();
        a.save_new(&db).unwrap();
        let loaded = Announcement::load(&db, a.id).unwrap().unwrap();
        assert_eq!(loaded.title, "Welcome");
        assert_eq!(loaded.slide_count(), 1);
    }
}
