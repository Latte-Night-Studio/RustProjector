use crate::color::RgbColor;
use crate::database::Database;
use crate::error::Result;
use crate::font::FontSpec;
use rusqlite::{params, OptionalExtension};

/// Song categories used by the original application.  They are translated in
/// the Qt version; the English strings are kept here and category ids are the
/// index in this list.
pub const CATEGORIES: &[&str] = &[
    "Other",
    "Bible Stories",
    "Gospel",
    "God, His love and greatness",
    "The Resurrection of Christ",
    "Time",
    "The second coming of Christ and the judgement",
    "Children and Family",
    "For new converts",
    "Spiritual struggle and victory",
    "Harvest",
    "Jesus Christ",
    "Love",
    "Mother",
    "Prayer",
    "Youth",
    "Wedding",
    "Sunset / Sunrise",
    "Baptism",
    "New Years",
    "Funeral",
    "At the ordination",
    "On the Lord's Supper",
    "Heavenly abode",
    "Instruction and self-test",
    "Holy Ghost",
    "Church",
    "Before church meeting",
    "Last Days",
    "Practical life with God",
    "At the end of church meeting",
    "Welcome and farewell",
    "The call to work",
    "Call to repentance",
    "Journey of faith, faith and hope",
    "Various Christian holidays",
    "Determination and faithfulness",
    "Christmas",
    "Following Christ",
    "The Word of God",
    "Salvation",
    "Suffering and death of Christ",
    "Consolation and encouragement",
    "Praise and thanksgiving",
    "Christian Joy",
];

pub fn category_name(id: i32) -> &'static str {
    CATEGORIES
        .get(id.max(0) as usize)
        .copied()
        .unwrap_or("Other")
}

/// Remove all non-alphanumeric characters, replacing them with spaces.  This is
/// the `clean()` helper from the original `song.cpp`.
pub fn clean(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
        } else {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn is_stanza_verse_title(s: &str) -> bool {
    ["Verse", "Куплет", "Strophe", "Verš"]
        .iter()
        .any(|p| s.starts_with(p))
}

pub fn is_stanza_and_verse_title(s: &str) -> bool {
    ["&Verse", "&Куплет", "&Strophe", "&Verš"]
        .iter()
        .any(|p| s.starts_with(p))
}

pub fn is_stanza_refrain_title(s: &str) -> bool {
    ["Chorus", "Sbor", "Refrain", "Припев", "Приспів", "Refrén"]
        .iter()
        .any(|p| s.starts_with(p))
}

pub fn is_stanza_and_refrain_title(s: &str) -> bool {
    [
        "&Chorus",
        "&Sbor",
        "&Refrain",
        "&Припев",
        "&Приспів",
        "&Refrén",
    ]
    .iter()
    .any(|p| s.starts_with(p))
}

pub fn is_stanza_slide_title(s: &str) -> bool {
    [
        "Slide",
        "Слайд",
        "Dia",
        "Snímek",
        "Insert",
        "Вставка",
        "Einfügung",
        "Vložka",
        "Intro",
        "Вступление",
        "Einleitung",
        "Úvod",
        "Ending",
        "Окончание",
        "Закінчення",
        "Ende",
        "Závěr",
    ]
    .iter()
    .any(|p| s.starts_with(p))
}

pub fn is_stanza_title(s: &str) -> bool {
    is_stanza_verse_title(s)
        || is_stanza_and_verse_title(s)
        || is_stanza_refrain_title(s)
        || is_stanza_and_refrain_title(s)
        || is_stanza_slide_title(s)
}

#[derive(Clone, Debug, Default)]
pub struct Songbook {
    pub id: i64,
    pub name: String,
    pub info: String,
}

impl Songbook {
    pub fn load_all(db: &Database) -> Result<Vec<Songbook>> {
        let conn = db.conn();
        let mut stmt = conn.prepare("SELECT id, name, info FROM Songbooks ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(Songbook {
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

    pub fn add(db: &Database, name: &str, info: &str) -> Result<i64> {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO Songbooks (name, info) VALUES (?1, ?2)",
            params![name.trim(), info],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update(db: &Database, id: i64, name: &str, info: &str) -> Result<()> {
        db.conn().execute(
            "UPDATE Songbooks SET name = ?1, info = ?2 WHERE id = ?3",
            params![name.trim(), info, id],
        )?;
        Ok(())
    }

    pub fn delete(db: &Database, id: i64) -> Result<()> {
        let conn = db.conn();
        conn.execute("DELETE FROM Songbooks WHERE id = ?1", params![id])?;
        conn.execute(
            "DELETE FROM Songs WHERE songbook_id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }
}

/// A single stanza prepared for projection.
#[derive(Clone, Debug)]
pub struct Stanza {
    pub number: i32,
    pub stanza: String,
    pub stanza_title: String,
    pub words_by: String,
    pub music_by: String,
    pub tune: String,
    pub is_last: bool,
    pub use_private_settings: bool,
    pub alignment_v: i32,
    pub alignment_h: i32,
    pub color: RgbColor,
    pub font: FontSpec,
    pub info_color: RgbColor,
    pub info_font: FontSpec,
    pub ending_color: RgbColor,
    pub ending_font: FontSpec,
    pub use_background: bool,
    pub background_name: String,
    pub background: Option<Vec<u8>>,
}

/// A song row from the `Songs` table.
#[derive(Clone, Debug)]
pub struct Song {
    pub song_id: i64,
    /// Songbooks are referenced by a text id in the original database even
    /// though the column is declared INTEGER.
    pub songbook_id: String,
    pub songbook_name: String,
    pub number: i32,
    pub title: String,
    pub category: i32,
    pub tune: String,
    pub words_by: String,
    pub music_by: String,
    pub song_text: String,
    pub notes: String,
    pub use_private_settings: bool,
    pub alignment_v: i32,
    pub alignment_h: i32,
    pub color: RgbColor,
    pub font: FontSpec,
    pub info_color: RgbColor,
    pub info_font: FontSpec,
    pub ending_color: RgbColor,
    pub ending_font: FontSpec,
    pub use_background: bool,
    pub background_name: String,
    pub background: Option<Vec<u8>>,
    pub count: i64,
    pub date: String,
}

impl Default for Song {
    fn default() -> Self {
        Self {
            song_id: 0,
            songbook_id: String::new(),
            songbook_name: String::new(),
            number: 1,
            title: String::new(),
            category: 0,
            tune: String::new(),
            words_by: String::new(),
            music_by: String::new(),
            song_text: String::new(),
            notes: String::new(),
            use_private_settings: false,
            alignment_v: 1,
            alignment_h: 1,
            color: RgbColor::WHITE,
            font: FontSpec::new("Arial", 28),
            info_color: RgbColor::WHITE,
            info_font: FontSpec::new("Arial", 28),
            ending_color: RgbColor::WHITE,
            ending_font: FontSpec::new("Arial", 28),
            use_background: false,
            background_name: String::new(),
            background: None,
            count: 0,
            date: String::new(),
        }
    }
}

impl Song {
    pub fn new(id: i64) -> Self {
        Self {
            song_id: id,
            ..Default::default()
        }
    }

    pub fn is_valid(&self) -> bool {
        self.song_id > 0
    }

    pub fn category_name(&self) -> &'static str {
        category_name(self.category)
    }

    pub fn load_all(db: &Database) -> Result<Vec<Song>> {
        let conn = db.conn();
        let mut books = std::collections::HashMap::new();
        {
            let mut stmt = conn.prepare("SELECT id, name FROM Songbooks")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?.to_string(),
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                ))
            })?;
            for row in rows {
                let (id, name) = row?;
                books.insert(id, name);
            }
        }

        let mut stmt = conn.prepare(
            "SELECT id, songbook_id, number, title, category, tune, words, music, song_text, notes, \
                    use_private, alignment_v, alignment_h, color, font, info_color, info_font, ending_color, \
                    ending_font, use_background, background_name, background, count, date \
             FROM Songs ORDER BY number, title",
        )?;
        let rows = stmt.query_map([], song_from_row)?;
        let mut list = Vec::new();
        for row in rows {
            let mut song = row?;
            song.songbook_name = books.get(&song.songbook_id).cloned().unwrap_or_default();
            list.push(song);
        }
        Ok(list)
    }

    pub fn load(db: &Database, id: i64) -> Result<Option<Song>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, songbook_id, number, title, category, tune, words, music, song_text, notes, \
                    use_private, alignment_v, alignment_h, color, font, info_color, info_font, ending_color, \
                    ending_font, use_background, background_name, background, count, date \
             FROM Songs WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let mut song = song_from_row(row)?;
            song.songbook_name = conn
                .query_row(
                    "SELECT name FROM Songbooks WHERE id = ?1",
                    params![song.songbook_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()?
                .flatten()
                .unwrap_or_default();
            Ok(Some(song))
        } else {
            Ok(None)
        }
    }

    pub fn save(&mut self, db: &Database) -> Result<()> {
        if self.song_id > 0 {
            self.save_update(db)
        } else {
            self.save_new(db)
        }
    }

    pub fn save_new(&mut self, db: &Database) -> Result<()> {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO Songs (songbook_id, number, title, category, tune, words, music, song_text, notes, \
              use_private, alignment_v, alignment_h, color, font, info_color, info_font, ending_color, ending_font, \
              use_background, background_name, background, count, date) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23)",
            params![
                self.songbook_id,
                self.number,
                self.title,
                self.category,
                self.tune,
                self.words_by,
                self.music_by,
                self.song_text,
                self.notes,
                self.use_private_settings,
                self.alignment_v,
                self.alignment_h,
                self.color.to_qt() as i64,
                self.font.to_qt(),
                self.info_color.to_qt() as i64,
                self.info_font.to_qt(),
                self.ending_color.to_qt() as i64,
                self.ending_font.to_qt(),
                self.use_background,
                self.background_name,
                self.background,
                self.count,
                self.date
            ],
        )?;
        self.song_id = conn.last_insert_rowid();
        Ok(())
    }

    pub fn save_update(&self, db: &Database) -> Result<()> {
        db.conn().execute(
            "UPDATE Songs SET songbook_id = ?1, number = ?2, title = ?3, category = ?4, tune = ?5, words = ?6, \
              music = ?7, song_text = ?8, notes = ?9, use_private = ?10, alignment_v = ?11, alignment_h = ?12, \
              color = ?13, font = ?14, info_color = ?15, info_font = ?16, ending_color = ?17, ending_font = ?18, \
              use_background = ?19, background_name = ?20, background = ?21, count = ?22, date = ?23 WHERE id = ?24",
            params![
                self.songbook_id,
                self.number,
                self.title,
                self.category,
                self.tune,
                self.words_by,
                self.music_by,
                self.song_text,
                self.notes,
                self.use_private_settings,
                self.alignment_v,
                self.alignment_h,
                self.color.to_qt() as i64,
                self.font.to_qt(),
                self.info_color.to_qt() as i64,
                self.info_font.to_qt(),
                self.ending_color.to_qt() as i64,
                self.ending_font.to_qt(),
                self.use_background,
                self.background_name,
                self.background,
                self.count,
                self.date,
                self.song_id
            ],
        )?;
        Ok(())
    }

    pub fn delete(db: &Database, id: i64) -> Result<()> {
        db.conn()
            .execute("DELETE FROM Songs WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn last_number(db: &Database, songbook_id: &str) -> Result<i32> {
        let conn = db.conn();
        let last: Option<i64> = conn
            .query_row(
                "SELECT MAX(number) FROM Songs WHERE songbook_id = ?1",
                params![songbook_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        Ok(last.unwrap_or(0) as i32 + 1)
    }

    /// Prepare the ordered list of stanzas, following the singing order rules
    /// of the original implementation (verses, refrain, inserts, `&` parts).
    pub fn text_list(&self) -> Vec<String> {
        let song_list: Vec<String> = self
            .song_text
            .replace("\r\n", "\n")
            .split('\n')
            .map(|s| s.to_string())
            .collect();

        let mut formatted: Vec<String> = Vec::new();
        let mut chorus: Vec<String> = Vec::new();
        let mut has_chorus = false;
        let mut has_vstavka = false;
        let mut pnum: usize = 0;
        let mut chor = 0;

        while pnum < song_list.len() {
            let line = song_list[pnum].clone();
            if is_stanza_verse_title(&line) {
                let text = Self::stanza_block(&mut pnum, &song_list);
                formatted.push(text);
                if has_chorus {
                    formatted.extend(chorus.iter().cloned());
                }
                has_vstavka = false;
            } else if is_stanza_and_verse_title(&line) {
                let text = Self::stanza_block(&mut pnum, &song_list);
                if has_chorus {
                    Self::remove_last_chorus(&chorus, &mut formatted);
                }
                formatted.push(text);
                if has_chorus {
                    formatted.extend(chorus.iter().cloned());
                }
                has_vstavka = false;
            } else if is_stanza_slide_title(&line) {
                let text = Self::stanza_block(&mut pnum, &song_list);
                formatted.push(text);
                has_vstavka = true;
            } else if is_stanza_refrain_title(&line) {
                let text = Self::stanza_block(&mut pnum, &song_list);
                let chorus_old = chorus.clone();
                chorus.clear();
                chorus.push(text);
                has_chorus = true;
                chor += 1;

                if chor == 1 {
                    formatted.extend(chorus.iter().cloned());
                } else if chor == 2 && !has_vstavka {
                    Self::remove_last_chorus(&chorus_old, &mut formatted);
                    formatted.extend(chorus.iter().cloned());
                    chor -= 1;
                } else if chor == 2 && has_vstavka {
                    formatted.extend(chorus.iter().cloned());
                    chor -= 1;
                }
                has_vstavka = false;
            } else if is_stanza_and_refrain_title(&line) {
                let text = Self::stanza_block(&mut pnum, &song_list);
                Self::remove_last_chorus(&chorus, &mut formatted);
                chorus.push(text);
                formatted.extend(chorus.iter().cloned());
            }
            pnum += 1;
        }

        if formatted.is_empty() && !self.song_text.trim().is_empty() {
            // A song without any recognised stanza titles is still shown as one
            // slide, matching how the original editor creates free-form songs.
            formatted.push(self.song_text.trim().to_string());
        }

        formatted
    }

    fn stanza_block(i: &mut usize, list: &[String]) -> String {
        let j = *i;
        let mut block = String::new();
        while *i < list.len() {
            let mut line = list[*i].clone();
            if line.starts_with('&') {
                line = line.replace('&', "");
            }
            if is_stanza_title(&line) && *i != j {
                *i = i.saturating_sub(1);
                break;
            }
            block.push_str(&line);
            block.push('\n');
            *i += 1;
        }
        block.trim().to_string()
    }

    fn remove_last_chorus(chorus: &[String], list: &mut Vec<String>) {
        for _ in 0..chorus.len() {
            list.pop();
        }
    }

    /// Return stanza `current` prepared for projection.
    pub fn stanza(&self, current: usize) -> Stanza {
        let song_list = self.text_list();
        let index = current.min(song_list.len().saturating_sub(1));
        let raw = song_list.get(index).cloned().unwrap_or_default();
        let mut lines: Vec<String> = raw.split('\n').map(|s| s.to_string()).collect();

        let mut stanza_title = String::new();
        if let Some(first) = lines.first() {
            if is_stanza_title(first) {
                stanza_title = first.clone();
                lines.remove(0);
            }
        }

        Stanza {
            number: self.number,
            stanza: lines.join("\n").trim().to_string(),
            stanza_title,
            words_by: self.words_by.clone(),
            music_by: self.music_by.clone(),
            tune: self.tune.clone(),
            is_last: index + 1 == song_list.len(),
            use_private_settings: self.use_private_settings,
            alignment_v: self.alignment_v,
            alignment_h: self.alignment_h,
            color: self.color,
            font: self.font.clone(),
            info_color: self.info_color,
            info_font: self.info_font.clone(),
            ending_color: self.ending_color,
            ending_font: self.ending_font.clone(),
            use_background: self.use_background,
            background_name: self.background_name.clone(),
            background: self.background.clone(),
        }
    }
}

fn song_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Song> {
    let color = row.get::<_, Option<i64>>(13)?.unwrap_or(0x00ff_ffff) as u32;
    let info_color = row.get::<_, Option<i64>>(15)?.unwrap_or(0x00ff_ffff) as u32;
    let ending_color = row.get::<_, Option<i64>>(17)?.unwrap_or(0x00ff_ffff) as u32;
    let font_text = row.get::<_, Option<String>>(14)?.unwrap_or_default();
    let info_font_text = row.get::<_, Option<String>>(16)?.unwrap_or_default();
    let ending_font_text = row.get::<_, Option<String>>(18)?.unwrap_or_default();

    Ok(Song {
        song_id: row.get(0)?,
        songbook_id: match row.get::<_, Option<i64>>(1)? {
            Some(value) => value.to_string(),
            None => row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        },
        songbook_name: String::new(),
        number: row.get::<_, Option<i64>>(2)?.unwrap_or(1) as i32,
        title: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        category: row.get::<_, Option<i64>>(4)?.unwrap_or_default() as i32,
        tune: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
        words_by: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        music_by: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
        song_text: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
        notes: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
        use_private_settings: row.get::<_, Option<i64>>(10)?.unwrap_or_default() != 0,
        alignment_v: row.get::<_, Option<i64>>(11)?.unwrap_or_default() as i32,
        alignment_h: row.get::<_, Option<i64>>(12)?.unwrap_or_default() as i32,
        color: RgbColor::from_qt(color),
        font: if font_text.trim().is_empty() {
            FontSpec::default()
        } else {
            FontSpec::from_qt(&font_text)
        },
        info_color: RgbColor::from_qt(info_color),
        info_font: if info_font_text.trim().is_empty() {
            FontSpec::default()
        } else {
            FontSpec::from_qt(&info_font_text)
        },
        ending_color: RgbColor::from_qt(ending_color),
        ending_font: if ending_font_text.trim().is_empty() {
            FontSpec::default()
        } else {
            FontSpec::from_qt(&ending_font_text)
        },
        use_background: row.get::<_, Option<i64>>(19)?.unwrap_or_default() != 0,
        background_name: row.get::<_, Option<String>>(20)?.unwrap_or_default(),
        background: row.get::<_, Option<Vec<u8>>>(21)?,
        count: row.get::<_, Option<i64>>(22)?.unwrap_or_default(),
        date: row.get::<_, Option<String>>(23)?.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song_with_text(text: &str) -> Song {
        Song {
            song_text: text.into(),
            ..Default::default()
        }
    }

    #[test]
    fn verse_chorus_order() {
        let song = song_with_text("Verse 1\nline one\nChorus\nrefrain line\nVerse 2\nline two");
        let list = song.text_list();
        assert_eq!(list.len(), 4);
        assert!(list[0].contains("line one"));
        assert!(list[1].contains("refrain line"));
        assert!(list[2].contains("line two"));
        assert!(list[3].contains("refrain line"));
    }

    #[test]
    fn stanza_removes_title() {
        let song = song_with_text("Verse 1\nline one\nline two");
        let st = song.stanza(0);
        assert_eq!(st.stanza_title, "Verse 1");
        assert_eq!(st.stanza, "line one\nline two");
        assert!(st.is_last);
    }

    #[test]
    fn clean_removes_punctuation() {
        assert_eq!(clean("Hello,   world!"), "Hello world");
    }
}
