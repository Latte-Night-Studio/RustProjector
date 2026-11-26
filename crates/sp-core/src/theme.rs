use crate::color::RgbColor;
use crate::database::Database;
use crate::error::Result;
use crate::font::FontSpec;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PassiveSettings {
    pub use_background: bool,
    pub background_name: String,
    pub background: Option<Vec<u8>>,
    pub use_disp2_settings: bool,
}

impl Default for PassiveSettings {
    fn default() -> Self {
        Self {
            use_background: false,
            background_name: String::new(),
            background: None,
            use_disp2_settings: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BibleSettings {
    pub use_shadow: bool,
    pub use_fading: bool,
    pub use_blur_shadow: bool,
    pub use_background: bool,
    pub background_name: String,
    pub background: Option<Vec<u8>>,
    pub text_font: FontSpec,
    pub text_color: RgbColor,
    pub text_alignment_v: i32,
    pub text_alignment_h: i32,
    pub caption_font: FontSpec,
    pub caption_color: RgbColor,
    pub caption_alignment: i32,
    pub caption_position: i32,
    pub use_abbriviations: bool,
    pub screen_use: i32,
    pub screen_position: i32,
    pub use_disp2_settings: bool,
}

impl Default for BibleSettings {
    fn default() -> Self {
        Self {
            use_shadow: true,
            use_fading: true,
            use_blur_shadow: false,
            use_background: false,
            background_name: String::new(),
            background: None,
            text_font: FontSpec::new("Arial", 48),
            text_color: RgbColor::WHITE,
            text_alignment_v: 0,
            text_alignment_h: 0,
            caption_font: FontSpec::new("Arial", 36),
            caption_color: RgbColor::WHITE,
            caption_alignment: 2,
            caption_position: 1,
            use_abbriviations: false,
            screen_use: 100,
            screen_position: 1,
            use_disp2_settings: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SongSettings {
    pub use_shadow: bool,
    pub use_fading: bool,
    pub use_blur_shadow: bool,
    pub show_stanza_title: bool,
    pub show_song_key: bool,
    pub show_song_number: bool,
    pub info_color: RgbColor,
    pub info_font: FontSpec,
    pub info_align: i32,
    pub show_song_ending: bool,
    pub ending_color: RgbColor,
    pub ending_font: FontSpec,
    pub ending_type: i32,
    pub ending_position: i32,
    pub use_background: bool,
    pub background_name: String,
    pub background: Option<Vec<u8>>,
    pub text_color: RgbColor,
    pub text_font: FontSpec,
    pub text_alignment_v: i32,
    pub text_alignment_h: i32,
    pub screen_use: i32,
    pub screen_position: i32,
    pub use_disp2_settings: bool,
}

impl Default for SongSettings {
    fn default() -> Self {
        Self {
            use_shadow: true,
            use_fading: true,
            use_blur_shadow: false,
            show_stanza_title: false,
            show_song_key: false,
            show_song_number: false,
            info_color: RgbColor::WHITE,
            info_font: FontSpec::new("Arial", 36),
            info_align: 0,
            show_song_ending: true,
            ending_color: RgbColor::WHITE,
            ending_font: FontSpec::new("Arial", 48),
            ending_type: 0,
            ending_position: 1,
            use_background: false,
            background_name: String::new(),
            background: None,
            text_color: RgbColor::WHITE,
            text_font: FontSpec::new("Arial", 48),
            text_alignment_v: 1,
            text_alignment_h: 1,
            screen_use: 100,
            screen_position: 1,
            use_disp2_settings: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnnounceSettings {
    pub use_shadow: bool,
    pub use_fading: bool,
    pub use_blur_shadow: bool,
    pub use_background: bool,
    pub background_name: String,
    pub background: Option<Vec<u8>>,
    pub text_font: FontSpec,
    pub text_color: RgbColor,
    pub text_alignment_v: i32,
    pub text_alignment_h: i32,
    pub use_disp2_settings: bool,
}

impl Default for AnnounceSettings {
    fn default() -> Self {
        Self {
            use_shadow: true,
            use_fading: true,
            use_blur_shadow: false,
            use_background: false,
            background_name: String::new(),
            background: None,
            text_font: FontSpec::new("Arial", 48),
            text_color: RgbColor::WHITE,
            text_alignment_v: 0,
            text_alignment_h: 1,
            use_disp2_settings: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeInfo {
    pub theme_id: i64,
    pub name: String,
    pub comments: String,
}

impl Default for ThemeInfo {
    fn default() -> Self {
        Self {
            theme_id: 0,
            name: "Default".into(),
            comments: "Default SoftProjector Theme".into(),
        }
    }
}

/// A complete theme with separate settings for both display outputs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Theme {
    pub theme_id: i64,
    pub name: String,
    pub comments: String,
    pub passive: PassiveSettings,
    pub passive2: PassiveSettings,
    pub bible: BibleSettings,
    pub bible2: BibleSettings,
    pub song: SongSettings,
    pub song2: SongSettings,
    pub announce: AnnounceSettings,
    pub announce2: AnnounceSettings,
}

impl Default for Theme {
    fn default() -> Self {
        let info = ThemeInfo::default();
        Self {
            theme_id: info.theme_id,
            name: info.name,
            comments: info.comments,
            passive: PassiveSettings::default(),
            passive2: PassiveSettings::default(),
            bible: BibleSettings::default(),
            bible2: BibleSettings::default(),
            song: SongSettings::default(),
            song2: SongSettings::default(),
            announce: AnnounceSettings::default(),
            announce2: AnnounceSettings::default(),
        }
    }
}

impl Theme {
    pub fn info(&self) -> ThemeInfo {
        ThemeInfo {
            theme_id: self.theme_id,
            name: self.name.clone(),
            comments: self.comments.clone(),
        }
    }

    /// Load a theme.  When `theme_id` does not exist the first theme is loaded,
    /// and when the database contains no themes at all a default theme is
    /// created.
    pub fn load(db: &Database, theme_id: i64) -> Result<Self> {
        let conn = db.conn();
        let mut theme = Theme::default();
        let found: Option<(i64, String, String)> = conn
            .query_row(
                "SELECT id, name, comment FROM Themes WHERE id = ?1",
                params![theme_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        let found = match found {
            Some(value) => Some(value),
            None => {
                let first = conn
                    .query_row(
                        "SELECT id, name, comment FROM Themes ORDER BY id LIMIT 1",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .optional()?;
                if first.is_some() {
                    first
                } else {
                    theme.save_new(db)?;
                    Some((theme.theme_id, theme.name.clone(), theme.comments.clone()))
                }
            }
        };

        if let Some((id, name, comments)) = found {
            theme.theme_id = id;
            theme.name = name;
            theme.comments = comments;
            theme.load_sections(conn)?;
        }
        Ok(theme)
    }

    /// Ensure at least one theme exists and return its id.
    pub fn ensure_default(db: &Database) -> Result<i64> {
        let conn = db.conn();
        let existing: Option<i64> = conn
            .query_row("SELECT id FROM Themes ORDER BY id LIMIT 1", [], |row| {
                row.get(0)
            })
            .optional()?;
        if let Some(id) = existing {
            Ok(id)
        } else {
            let mut theme = Theme::default();
            theme.save_new(db)?;
            Ok(theme.theme_id)
        }
    }

    pub fn list(db: &Database) -> Result<Vec<ThemeInfo>> {
        let conn = db.conn();
        let mut stmt = conn.prepare("SELECT id, name, comment FROM Themes ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            Ok(ThemeInfo {
                theme_id: row.get(0)?,
                name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                comments: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn save_new(&mut self, db: &Database) -> Result<()> {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO Themes (name, comment) VALUES (?1, ?2)",
            params![self.name, self.comments],
        )?;
        self.theme_id = conn.last_insert_rowid();
        self.insert_sections(conn)?;
        Ok(())
    }

    pub fn save_update(&self, db: &Database) -> Result<()> {
        let conn = db.conn();
        conn.execute(
            "UPDATE Themes SET name = ?1, comment = ?2 WHERE id = ?3",
            params![self.name, self.comments, self.theme_id],
        )?;
        self.update_sections(conn)?;
        Ok(())
    }

    /// Save a theme under a new name/id, keeping the source theme untouched.
    pub fn duplicate(&self, db: &Database, name: &str, comments: &str) -> Result<i64> {
        let mut copy = self.clone();
        copy.theme_id = 0;
        copy.name = name.to_string();
        copy.comments = comments.to_string();
        copy.save_new(db)?;
        Ok(copy.theme_id)
    }

    pub fn delete(db: &Database, theme_id: i64) -> Result<()> {
        let conn = db.conn();
        conn.execute("DELETE FROM Themes WHERE id = ?1", params![theme_id])?;
        for table in ["ThemeAnnounce", "ThemeBible", "ThemePassive", "ThemeSong"] {
            conn.execute(
                &format!("DELETE FROM {} WHERE theme_id = ?1", table),
                params![theme_id],
            )?;
        }
        Ok(())
    }

    fn load_sections(&mut self, conn: &Connection) -> Result<()> {
        self.passive = load_passive(conn, self.theme_id, 1)?.unwrap_or_default();
        self.passive2 = load_passive(conn, self.theme_id, 2)?.unwrap_or_default();
        self.bible = load_bible(conn, self.theme_id, 1)?.unwrap_or_default();
        self.bible2 = load_bible(conn, self.theme_id, 2)?.unwrap_or_default();
        self.song = load_song(conn, self.theme_id, 1)?.unwrap_or_default();
        self.song2 = load_song(conn, self.theme_id, 2)?.unwrap_or_default();
        self.announce = load_announce(conn, self.theme_id, 1)?.unwrap_or_default();
        self.announce2 = load_announce(conn, self.theme_id, 2)?.unwrap_or_default();
        Ok(())
    }

    fn insert_sections(&self, conn: &Connection) -> Result<()> {
        for (disp, settings) in [(1, &self.passive), (2, &self.passive2)] {
            insert_passive(conn, self.theme_id, disp, settings)?;
        }
        for (disp, settings) in [(1, &self.bible), (2, &self.bible2)] {
            insert_bible(conn, self.theme_id, disp, settings)?;
        }
        for (disp, settings) in [(1, &self.song), (2, &self.song2)] {
            insert_song(conn, self.theme_id, disp, settings)?;
        }
        for (disp, settings) in [(1, &self.announce), (2, &self.announce2)] {
            insert_announce(conn, self.theme_id, disp, settings)?;
        }
        Ok(())
    }

    fn update_sections(&self, conn: &Connection) -> Result<()> {
        for (disp, settings) in [(1, &self.passive), (2, &self.passive2)] {
            update_passive(conn, self.theme_id, disp, settings)?;
        }
        for (disp, settings) in [(1, &self.bible), (2, &self.bible2)] {
            update_bible(conn, self.theme_id, disp, settings)?;
        }
        for (disp, settings) in [(1, &self.song), (2, &self.song2)] {
            update_song(conn, self.theme_id, disp, settings)?;
        }
        for (disp, settings) in [(1, &self.announce), (2, &self.announce2)] {
            update_announce(conn, self.theme_id, disp, settings)?;
        }
        Ok(())
    }
}

fn color_to_i64(color: RgbColor) -> i64 {
    color.to_qt() as i64
}

fn color_from_i64(value: Option<i64>) -> RgbColor {
    RgbColor::from_qt(value.unwrap_or(0x00ff_ffff) as u32)
}

fn load_passive(conn: &Connection, theme_id: i64, disp: i32) -> Result<Option<PassiveSettings>> {
    let mut stmt = conn.prepare(
        "SELECT use_background, background_name, background, use_disp_2 \
         FROM ThemePassive WHERE theme_id = ?1 AND disp = ?2",
    )?;
    let mut rows = stmt.query(params![theme_id, disp])?;
    if let Some(row) = rows.next()? {
        Ok(Some(PassiveSettings {
            use_background: row.get::<_, Option<i64>>(0)?.unwrap_or(0) != 0,
            background_name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            background: row.get::<_, Option<Vec<u8>>>(2)?,
            use_disp2_settings: row.get::<_, Option<i64>>(3)?.unwrap_or(0) != 0,
        }))
    } else {
        Ok(None)
    }
}

fn load_bible(conn: &Connection, theme_id: i64, disp: i32) -> Result<Option<BibleSettings>> {
    let mut stmt = conn.prepare(
        "SELECT use_shadow, use_fading, use_blur_shadow, use_background, background_name, background, \
                text_font, text_color, text_align_v, text_align_h, caption_font, caption_color, caption_align, \
                caption_position, use_abbr, screen_use, screen_position, use_disp_2 \
         FROM ThemeBible WHERE theme_id = ?1 AND disp = ?2",
    )?;
    let mut rows = stmt.query(params![theme_id, disp])?;
    if let Some(row) = rows.next()? {
        let defaults = BibleSettings::default();
        Ok(Some(BibleSettings {
            use_shadow: row.get::<_, Option<i64>>(0)?.unwrap_or(1) != 0,
            use_fading: row.get::<_, Option<i64>>(1)?.unwrap_or(1) != 0,
            use_blur_shadow: row.get::<_, Option<i64>>(2)?.unwrap_or(0) != 0,
            use_background: row.get::<_, Option<i64>>(3)?.unwrap_or(0) != 0,
            background_name: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            background: row.get::<_, Option<Vec<u8>>>(5)?,
            text_font: FontSpec::from_qt(
                &row.get::<_, Option<String>>(6)?
                    .unwrap_or_else(|| defaults.text_font.to_qt()),
            ),
            text_color: color_from_i64(row.get(7)?),
            text_alignment_v: row.get::<_, Option<i64>>(8)?.unwrap_or(0) as i32,
            text_alignment_h: row.get::<_, Option<i64>>(9)?.unwrap_or(0) as i32,
            caption_font: FontSpec::from_qt(
                &row.get::<_, Option<String>>(10)?
                    .unwrap_or_else(|| defaults.caption_font.to_qt()),
            ),
            caption_color: color_from_i64(row.get(11)?),
            caption_alignment: row.get::<_, Option<i64>>(12)?.unwrap_or(2) as i32,
            caption_position: row.get::<_, Option<i64>>(13)?.unwrap_or(1) as i32,
            use_abbriviations: row.get::<_, Option<i64>>(14)?.unwrap_or(0) != 0,
            screen_use: row.get::<_, Option<i64>>(15)?.unwrap_or(100) as i32,
            screen_position: row.get::<_, Option<i64>>(16)?.unwrap_or(1) as i32,
            use_disp2_settings: row.get::<_, Option<i64>>(17)?.unwrap_or(0) != 0,
        }))
    } else {
        Ok(None)
    }
}

fn load_song(conn: &Connection, theme_id: i64, disp: i32) -> Result<Option<SongSettings>> {
    let mut stmt = conn.prepare(
        "SELECT use_shadow, use_fading, use_blur_shadow, show_stanza_title, show_key, show_number, \
                info_color, info_font, info_align, show_song_ending, ending_color, ending_font, ending_type, \
                ending_position, use_background, background_name, background, text_font, text_color, \
                text_align_v, text_align_h, screen_use, screen_position, use_disp_2 \
         FROM ThemeSong WHERE theme_id = ?1 AND disp = ?2",
    )?;
    let mut rows = stmt.query(params![theme_id, disp])?;
    if let Some(row) = rows.next()? {
        let defaults = SongSettings::default();
        Ok(Some(SongSettings {
            use_shadow: row.get::<_, Option<i64>>(0)?.unwrap_or(1) != 0,
            use_fading: row.get::<_, Option<i64>>(1)?.unwrap_or(1) != 0,
            use_blur_shadow: row.get::<_, Option<i64>>(2)?.unwrap_or(0) != 0,
            show_stanza_title: row.get::<_, Option<i64>>(3)?.unwrap_or(0) != 0,
            show_song_key: row.get::<_, Option<i64>>(4)?.unwrap_or(0) != 0,
            show_song_number: row.get::<_, Option<i64>>(5)?.unwrap_or(0) != 0,
            info_color: color_from_i64(row.get(6)?),
            info_font: FontSpec::from_qt(
                &row.get::<_, Option<String>>(7)?
                    .unwrap_or_else(|| defaults.info_font.to_qt()),
            ),
            info_align: row.get::<_, Option<i64>>(8)?.unwrap_or(0) as i32,
            show_song_ending: row.get::<_, Option<i64>>(9)?.unwrap_or(1) != 0,
            ending_color: color_from_i64(row.get(10)?),
            ending_font: FontSpec::from_qt(
                &row.get::<_, Option<String>>(11)?
                    .unwrap_or_else(|| defaults.ending_font.to_qt()),
            ),
            ending_type: row.get::<_, Option<i64>>(12)?.unwrap_or(0) as i32,
            ending_position: row.get::<_, Option<i64>>(13)?.unwrap_or(1) as i32,
            use_background: row.get::<_, Option<i64>>(14)?.unwrap_or(0) != 0,
            background_name: row.get::<_, Option<String>>(15)?.unwrap_or_default(),
            background: row.get::<_, Option<Vec<u8>>>(16)?,
            text_font: FontSpec::from_qt(
                &row.get::<_, Option<String>>(17)?
                    .unwrap_or_else(|| defaults.text_font.to_qt()),
            ),
            text_color: color_from_i64(row.get(18)?),
            text_alignment_v: row.get::<_, Option<i64>>(19)?.unwrap_or(1) as i32,
            text_alignment_h: row.get::<_, Option<i64>>(20)?.unwrap_or(1) as i32,
            screen_use: row.get::<_, Option<i64>>(21)?.unwrap_or(100) as i32,
            screen_position: row.get::<_, Option<i64>>(22)?.unwrap_or(1) as i32,
            use_disp2_settings: row.get::<_, Option<i64>>(23)?.unwrap_or(0) != 0,
        }))
    } else {
        Ok(None)
    }
}

fn load_announce(conn: &Connection, theme_id: i64, disp: i32) -> Result<Option<AnnounceSettings>> {
    let mut stmt = conn.prepare(
        "SELECT use_shadow, use_fading, use_blur_shadow, use_background, background_name, background, \
                text_font, text_color, text_align_v, text_align_h, use_disp_2 \
         FROM ThemeAnnounce WHERE theme_id = ?1 AND disp = ?2",
    )?;
    let mut rows = stmt.query(params![theme_id, disp])?;
    if let Some(row) = rows.next()? {
        let defaults = AnnounceSettings::default();
        Ok(Some(AnnounceSettings {
            use_shadow: row.get::<_, Option<i64>>(0)?.unwrap_or(1) != 0,
            use_fading: row.get::<_, Option<i64>>(1)?.unwrap_or(1) != 0,
            use_blur_shadow: row.get::<_, Option<i64>>(2)?.unwrap_or(0) != 0,
            use_background: row.get::<_, Option<i64>>(3)?.unwrap_or(0) != 0,
            background_name: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            background: row.get::<_, Option<Vec<u8>>>(5)?,
            text_font: FontSpec::from_qt(
                &row.get::<_, Option<String>>(6)?
                    .unwrap_or_else(|| defaults.text_font.to_qt()),
            ),
            text_color: color_from_i64(row.get(7)?),
            text_alignment_v: row.get::<_, Option<i64>>(8)?.unwrap_or(0) as i32,
            text_alignment_h: row.get::<_, Option<i64>>(9)?.unwrap_or(1) as i32,
            use_disp2_settings: row.get::<_, Option<i64>>(10)?.unwrap_or(0) != 0,
        }))
    } else {
        Ok(None)
    }
}

fn insert_passive(conn: &Connection, theme_id: i64, disp: i32, s: &PassiveSettings) -> Result<()> {
    conn.execute(
        "INSERT INTO ThemePassive (theme_id, disp, use_background, background_name, background, use_disp_2) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            theme_id,
            disp,
            s.use_background,
            s.background_name,
            s.background,
            s.use_disp2_settings
        ],
    )?;
    Ok(())
}

fn insert_bible(conn: &Connection, theme_id: i64, disp: i32, s: &BibleSettings) -> Result<()> {
    conn.execute(
        "INSERT INTO ThemeBible (theme_id, disp, use_shadow, use_fading, use_blur_shadow, use_background, \
          background_name, background, text_font, text_color, text_align_v, text_align_h, caption_font, \
          caption_color, caption_align, caption_position, use_abbr, screen_use, screen_position, use_disp_2) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
        params![
            theme_id,
            disp,
            s.use_shadow,
            s.use_fading,
            s.use_blur_shadow,
            s.use_background,
            s.background_name,
            s.background,
            s.text_font.to_qt(),
            color_to_i64(s.text_color),
            s.text_alignment_v,
            s.text_alignment_h,
            s.caption_font.to_qt(),
            color_to_i64(s.caption_color),
            s.caption_alignment,
            s.caption_position,
            s.use_abbriviations,
            s.screen_use,
            s.screen_position,
            s.use_disp2_settings
        ],
    )?;
    Ok(())
}

fn insert_song(conn: &Connection, theme_id: i64, disp: i32, s: &SongSettings) -> Result<()> {
    conn.execute(
        "INSERT INTO ThemeSong (theme_id, disp, use_shadow, use_fading, use_blur_shadow, show_stanza_title, \
          show_key, show_number, info_color, info_font, info_align, show_song_ending, ending_color, ending_font, \
          ending_type, ending_position, use_background, background_name, background, text_font, text_color, \
          text_align_v, text_align_h, screen_use, screen_position, use_disp_2) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
        params![
            theme_id,
            disp,
            s.use_shadow,
            s.use_fading,
            s.use_blur_shadow,
            s.show_stanza_title,
            s.show_song_key,
            s.show_song_number,
            color_to_i64(s.info_color),
            s.info_font.to_qt(),
            s.info_align,
            s.show_song_ending,
            color_to_i64(s.ending_color),
            s.ending_font.to_qt(),
            s.ending_type,
            s.ending_position,
            s.use_background,
            s.background_name,
            s.background,
            s.text_font.to_qt(),
            color_to_i64(s.text_color),
            s.text_alignment_v,
            s.text_alignment_h,
            s.screen_use,
            s.screen_position,
            s.use_disp2_settings
        ],
    )?;
    Ok(())
}

fn insert_announce(
    conn: &Connection,
    theme_id: i64,
    disp: i32,
    s: &AnnounceSettings,
) -> Result<()> {
    conn.execute(
        "INSERT INTO ThemeAnnounce (theme_id, disp, use_shadow, use_fading, use_blur_shadow, use_background, \
          background_name, background, text_font, text_color, text_align_v, text_align_h, use_disp_2) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            theme_id,
            disp,
            s.use_shadow,
            s.use_fading,
            s.use_blur_shadow,
            s.use_background,
            s.background_name,
            s.background,
            s.text_font.to_qt(),
            color_to_i64(s.text_color),
            s.text_alignment_v,
            s.text_alignment_h,
            s.use_disp2_settings
        ],
    )?;
    Ok(())
}

fn update_passive(conn: &Connection, theme_id: i64, disp: i32, s: &PassiveSettings) -> Result<()> {
    let changed = conn.execute(
        "UPDATE ThemePassive SET use_background = ?1, background_name = ?2, background = ?3, use_disp_2 = ?4 \
         WHERE theme_id = ?5 AND disp = ?6",
        params![
            s.use_background,
            s.background_name,
            s.background,
            s.use_disp2_settings,
            theme_id,
            disp
        ],
    )?;
    if changed == 0 {
        insert_passive(conn, theme_id, disp, s)?;
    }
    Ok(())
}

fn update_bible(conn: &Connection, theme_id: i64, disp: i32, s: &BibleSettings) -> Result<()> {
    let changed = conn.execute(
        "UPDATE ThemeBible SET use_shadow=?1, use_fading=?2, use_blur_shadow=?3, use_background=?4, \
          background_name=?5, background=?6, text_font=?7, text_color=?8, text_align_v=?9, text_align_h=?10, \
          caption_font=?11, caption_color=?12, caption_align=?13, caption_position=?14, use_abbr=?15, \
          screen_use=?16, screen_position=?17, use_disp_2=?18 WHERE theme_id=?19 AND disp=?20",
        params![
            s.use_shadow,
            s.use_fading,
            s.use_blur_shadow,
            s.use_background,
            s.background_name,
            s.background,
            s.text_font.to_qt(),
            color_to_i64(s.text_color),
            s.text_alignment_v,
            s.text_alignment_h,
            s.caption_font.to_qt(),
            color_to_i64(s.caption_color),
            s.caption_alignment,
            s.caption_position,
            s.use_abbriviations,
            s.screen_use,
            s.screen_position,
            s.use_disp2_settings,
            theme_id,
            disp
        ],
    )?;
    if changed == 0 {
        insert_bible(conn, theme_id, disp, s)?;
    }
    Ok(())
}

fn update_song(conn: &Connection, theme_id: i64, disp: i32, s: &SongSettings) -> Result<()> {
    let changed = conn.execute(
        "UPDATE ThemeSong SET use_shadow=?1, use_fading=?2, use_blur_shadow=?3, show_stanza_title=?4, \
          show_key=?5, show_number=?6, info_color=?7, info_font=?8, info_align=?9, show_song_ending=?10, \
          ending_color=?11, ending_font=?12, ending_type=?13, ending_position=?14, use_background=?15, \
          background_name=?16, background=?17, text_font=?18, text_color=?19, text_align_v=?20, text_align_h=?21, \
          screen_use=?22, screen_position=?23, use_disp_2=?24 WHERE theme_id=?25 AND disp=?26",
        params![
            s.use_shadow,
            s.use_fading,
            s.use_blur_shadow,
            s.show_stanza_title,
            s.show_song_key,
            s.show_song_number,
            color_to_i64(s.info_color),
            s.info_font.to_qt(),
            s.info_align,
            s.show_song_ending,
            color_to_i64(s.ending_color),
            s.ending_font.to_qt(),
            s.ending_type,
            s.ending_position,
            s.use_background,
            s.background_name,
            s.background,
            s.text_font.to_qt(),
            color_to_i64(s.text_color),
            s.text_alignment_v,
            s.text_alignment_h,
            s.screen_use,
            s.screen_position,
            s.use_disp2_settings,
            theme_id,
            disp
        ],
    )?;
    if changed == 0 {
        insert_song(conn, theme_id, disp, s)?;
    }
    Ok(())
}

fn update_announce(
    conn: &Connection,
    theme_id: i64,
    disp: i32,
    s: &AnnounceSettings,
) -> Result<()> {
    let changed = conn.execute(
        "UPDATE ThemeAnnounce SET use_shadow=?1, use_fading=?2, use_blur_shadow=?3, use_background=?4, \
          background_name=?5, background=?6, text_font=?7, text_color=?8, text_align_v=?9, text_align_h=?10, \
          use_disp_2=?11 WHERE theme_id=?12 AND disp=?13",
        params![
            s.use_shadow,
            s.use_fading,
            s.use_blur_shadow,
            s.use_background,
            s.background_name,
            s.background,
            s.text_font.to_qt(),
            color_to_i64(s.text_color),
            s.text_alignment_v,
            s.text_alignment_h,
            s.use_disp2_settings,
            theme_id,
            disp
        ],
    )?;
    if changed == 0 {
        insert_announce(conn, theme_id, disp, s)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let mut theme = Theme::default();
        theme.name = "Test".into();
        theme.song.text_font.point_size = 61;
        theme.save_new(&db).unwrap();

        let loaded = Theme::load(&db, theme.theme_id).unwrap();
        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.song.text_font.point_size, 61);
        assert!(loaded.bible.caption_position == 1);
    }

    #[test]
    fn fallback_to_first_theme() {
        let db = Database::open_in_memory().unwrap();
        let mut theme = Theme::default();
        theme.name = "Only".into();
        theme.save_new(&db).unwrap();
        let loaded = Theme::load(&db, 999).unwrap();
        assert_eq!(loaded.name, "Only");
    }
}
