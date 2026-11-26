use crate::announcement::Announcement;
use crate::bible::BibleHistory;
use crate::color::RgbColor;
use crate::error::{Result, SpError};
use crate::font::FontSpec;
use crate::media::MediaItem;
use crate::slideshow::{SlideShow, SlideShowItem};
use crate::song::Song;
use rusqlite::types::ValueRef;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

/// A single entry of the projection schedule.
#[derive(Clone, Debug)]
pub struct ScheduleItem {
    /// Database id when the schedule has been saved, `-1` for new entries.
    pub scid: i64,
    pub name: String,
    pub kind: ScheduleKind,
}

#[derive(Clone, Debug)]
pub enum ScheduleKind {
    Bible(BibleHistory),
    Song(Song),
    SlideShow(SlideShow),
    Media(MediaItem),
    Announce(Announcement),
}

impl ScheduleItem {
    pub fn stype(&self) -> &'static str {
        match self.kind {
            ScheduleKind::Bible(_) => "bible",
            ScheduleKind::Song(_) => "song",
            ScheduleKind::SlideShow(_) => "slideshow",
            ScheduleKind::Media(_) => "media",
            ScheduleKind::Announce(_) => "announce",
        }
    }

    pub fn from_bible(history: BibleHistory) -> Self {
        let name = history.caption.clone();
        Self {
            scid: -1,
            name,
            kind: ScheduleKind::Bible(history),
        }
    }

    pub fn from_song(song: Song) -> Self {
        let name = format!("{} - {}", song.number, song.title);
        Self {
            scid: -1,
            name,
            kind: ScheduleKind::Song(song),
        }
    }

    pub fn from_slideshow(show: SlideShow) -> Self {
        let name = show.name.clone();
        Self {
            scid: -1,
            name,
            kind: ScheduleKind::SlideShow(show),
        }
    }

    pub fn from_media(media: MediaItem) -> Self {
        let name = media.short_path.clone();
        Self {
            scid: -1,
            name,
            kind: ScheduleKind::Media(media),
        }
    }

    pub fn from_announcement(announcement: Announcement) -> Self {
        let name = announcement.title.clone();
        Self {
            scid: -1,
            name,
            kind: ScheduleKind::Announce(announcement),
        }
    }
}

/// Save a schedule to a `.spsc` SQLite database, matching the schema of the
/// original application.
pub fn save_schedule(path: impl AsRef<Path>, schedule: &mut [ScheduleItem]) -> Result<()> {
    let path = path.as_ref();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let conn = Connection::open(path)?;
    create_schedule_schema(&conn)?;

    for (index, item) in schedule.iter_mut().enumerate() {
        conn.execute(
            "INSERT INTO schedule (stype, name, sorder) VALUES (?1, ?2, ?3)",
            params![item.stype(), item.name, (index + 1) as i64],
        )?;
        item.scid = conn.last_insert_rowid();
        save_item(&conn, item.scid, &item.kind)?;
    }
    Ok(())
}

fn create_schedule_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA user_version = 2;
        CREATE TABLE IF NOT EXISTS 'schedule' ('id' INTEGER PRIMARY KEY  AUTOINCREMENT  NOT NULL, 'stype' TEXT, 'name' TEXT, 'sorder' INTEGER );
        CREATE TABLE IF NOT EXISTS 'bible' ('scid' INTEGER, 'verseIds' TEXT, 'caption' TEXT, 'captionLong' TEXT);
        CREATE TABLE IF NOT EXISTS 'song' ('scid' INTEGER, 'songid' INTEGER, 'sbid' INTEGER, 'sbName' TEXT, 'number' INTEGER, 'title' TEXT, 'category' INTEGER, 'tune' TEXT, 'wordsBy' TEXT, 'musicBy' TEXT, 'songText' TEXT, 'notes' TEXT, 'usePrivate' BOOL, 'alignV' INTEGER, 'alignH' INTEGER, 'color' INTEGER, 'font' TEXT, 'infoColor' INTEGER, 'infoFont' TEXT, 'endingColor' INTEGER, 'endingFont' TEXT, 'useBack' BOOL, 'backImage' BLOB, 'backName' TEXT);
        CREATE TABLE IF NOT EXISTS 'slideshow' ('scid' INTEGER, 'ssid' INTEGER, 'name' TEXT, 'info' TEXT);
        CREATE TABLE IF NOT EXISTS 'slides' ('scid' INTEGER, 'sid' INTEGER, 'name' TEXT, 'path' TEXT, 'porder' INTEGER, 'image' BLOB, 'imageSmall' BLOB, 'imagePreview' BLOB);
        CREATE TABLE IF NOT EXISTS 'media' ('scid' INTEGER, 'name' TEXT, 'path' TEXT, 'aRatio' INTEGER);
        CREATE TABLE IF NOT EXISTS 'announce' ('scid' INTEGER, 'aId' INTEGER, 'title' TEXT, 'aText' TEXT, 'usePrivate' BOOL, 'useAuto' BOOL, 'loop' BOOL, 'slideTimer' INTEGER, 'font' TEXT, 'color' INTEGER, 'useBack' BOOL, 'backImage' BLOB, 'backPath' TEXT, 'alignV' INTEGER, 'alignH' INTEGER);
        "#,
    )?;
    Ok(())
}

fn save_item(conn: &Connection, scid: i64, kind: &ScheduleKind) -> Result<()> {
    match kind {
        ScheduleKind::Bible(history) => {
            conn.execute(
                "INSERT INTO bible (scid, verseIds, caption, captionLong) VALUES (?1,?2,?3,?4)",
                params![
                    scid,
                    history.verse_ids,
                    history.caption,
                    history.caption_long
                ],
            )?;
        }
        ScheduleKind::Song(song) => {
            conn.execute(
                "INSERT INTO song (scid, songid, sbid, sbName, number, title, category, tune, wordsBy, musicBy, \
                  songText, notes, usePrivate, alignV, alignH, color, font, infoColor, infoFont, endingColor, \
                  endingFont, useBack, backImage, backName) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)",
                params![
                    scid,
                    song.song_id,
                    song.songbook_id,
                    song.songbook_name,
                    song.number,
                    song.title,
                    song.category,
                    song.tune,
                    song.words_by,
                    song.music_by,
                    song.song_text,
                    song.notes,
                    song.use_private_settings,
                    song.alignment_v,
                    song.alignment_h,
                    song.color.to_qt() as i64,
                    song.font.to_qt(),
                    song.info_color.to_qt() as i64,
                    song.info_font.to_qt(),
                    song.ending_color.to_qt() as i64,
                    song.ending_font.to_qt(),
                    song.use_background,
                    song.background,
                    song.background_name
                ],
            )?;
        }
        ScheduleKind::SlideShow(show) => {
            conn.execute(
                "INSERT INTO slideshow (scid, ssid, name, info) VALUES (?1,?2,?3,?4)",
                params![scid, show.id, show.name, show.info],
            )?;
            for slide in &show.slides {
                conn.execute(
                    "INSERT INTO slides (scid, sid, name, path, porder, image, imageSmall, imagePreview) \
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                    params![
                        scid,
                        slide.slide_id,
                        slide.name,
                        slide.path,
                        slide.order,
                        slide.image,
                        slide.image_small,
                        slide.image_preview
                    ],
                )?;
            }
        }
        ScheduleKind::Media(media) => {
            conn.execute(
                "INSERT INTO media (scid, name, path, aRatio) VALUES (?1,?2,?3,?4)",
                params![scid, media.short_path, media.long_path, media.aspect_ratio],
            )?;
        }
        ScheduleKind::Announce(announcement) => {
            conn.execute(
                "INSERT INTO announce (scid, aId, title, aText, usePrivate, useAuto, loop, slideTimer, font, \
                  color, useBack, backImage, backPath, alignV, alignH) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                params![
                    scid,
                    announcement.id,
                    announcement.title,
                    announcement.text,
                    announcement.use_private_settings,
                    announcement.use_auto_next,
                    announcement.loop_slides,
                    announcement.slide_timer,
                    announcement.font.to_qt(),
                    announcement.color.to_qt() as i64,
                    announcement.use_background,
                    Option::<Vec<u8>>::None,
                    announcement.background_path,
                    announcement.alignment_v,
                    announcement.alignment_h
                ],
            )?;
        }
    }
    Ok(())
}

/// Read a `.spsc` schedule file.
pub fn open_schedule(path: impl AsRef<Path>) -> Result<Vec<ScheduleItem>> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(SpError::NotFound(path.to_path_buf()));
    }
    let conn = Connection::open(path)?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version != 2 {
        return Err(SpError::DatabaseVersion {
            found: version,
            expected: 2,
        });
    }

    let mut stmt = conn.prepare("SELECT id, stype, name FROM schedule ORDER BY sorder")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        ))
    })?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    drop(stmt);

    let mut schedule = Vec::new();
    for (scid, stype, name) in entries {
        let kind = match stype.as_str() {
            "bible" => open_bible(&conn, scid)?.map(ScheduleKind::Bible),
            "song" => open_song(&conn, scid)?.map(ScheduleKind::Song),
            "slideshow" => open_slideshow(&conn, scid)?.map(ScheduleKind::SlideShow),
            "media" => open_media(&conn, scid)?.map(ScheduleKind::Media),
            "announce" => open_announce(&conn, scid)?.map(ScheduleKind::Announce),
            _ => None,
        };
        if let Some(kind) = kind {
            schedule.push(ScheduleItem { scid, name, kind });
        }
    }
    Ok(schedule)
}

fn open_bible(conn: &Connection, scid: i64) -> Result<Option<BibleHistory>> {
    Ok(conn
        .query_row(
            "SELECT verseIds, caption, captionLong FROM bible WHERE scid = ?1 LIMIT 1",
            params![scid],
            |row| {
                Ok(BibleHistory {
                    verse_ids: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    caption: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    caption_long: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                })
            },
        )
        .optional()?)
}

fn open_song(conn: &Connection, scid: i64) -> Result<Option<Song>> {
    let mut stmt = conn.prepare(
        "SELECT songid, sbid, sbName, number, title, category, tune, wordsBy, musicBy, songText, notes, \
                usePrivate, alignV, alignH, color, font, infoColor, infoFont, endingColor, endingFont, useBack, \
                backImage, backName FROM song WHERE scid = ?1 LIMIT 1",
    )?;
    let mut rows = stmt.query(params![scid])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Song {
            song_id: row.get::<_, Option<i64>>(0)?.unwrap_or_default(),
            songbook_id: text_or_int(row, 1)?,
            songbook_name: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            number: row.get::<_, Option<i64>>(3)?.unwrap_or_default() as i32,
            title: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            category: row.get::<_, Option<i64>>(5)?.unwrap_or_default() as i32,
            tune: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
            words_by: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
            music_by: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            song_text: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
            notes: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
            use_private_settings: row.get::<_, Option<i64>>(11)?.unwrap_or_default() != 0,
            alignment_v: row.get::<_, Option<i64>>(12)?.unwrap_or_default() as i32,
            alignment_h: row.get::<_, Option<i64>>(13)?.unwrap_or_default() as i32,
            color: color_from_row(row.get(14)?),
            font: font_from_row(row.get(15)?),
            info_color: color_from_row(row.get(16)?),
            info_font: font_from_row(row.get(17)?),
            ending_color: color_from_row(row.get(18)?),
            ending_font: font_from_row(row.get(19)?),
            use_background: row.get::<_, Option<i64>>(20)?.unwrap_or_default() != 0,
            background: row.get::<_, Option<Vec<u8>>>(21)?,
            background_name: row.get::<_, Option<String>>(22)?.unwrap_or_default(),
            count: 0,
            date: String::new(),
        }))
    } else {
        Ok(None)
    }
}

fn open_slideshow(conn: &Connection, scid: i64) -> Result<Option<SlideShow>> {
    let mut stmt =
        conn.prepare("SELECT ssid, name, info FROM slideshow WHERE scid = ?1 LIMIT 1")?;
    let mut rows = stmt.query(params![scid])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let mut show = SlideShow {
        id: row.get::<_, Option<i64>>(0)?.unwrap_or_default(),
        name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        info: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        slides: Vec::new(),
    };
    drop(rows);
    drop(stmt);

    let mut stmt = conn.prepare(
        "SELECT sid, porder, name, path, image, imageSmall, imagePreview \
         FROM slides WHERE scid = ?1 ORDER BY porder",
    )?;
    let rows = stmt.query_map(params![scid], |row| {
        Ok(SlideShowItem {
            slide_id: row.get::<_, Option<i64>>(0)?.unwrap_or(-1),
            order: row.get::<_, Option<i64>>(1)?.unwrap_or_default() as i32,
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
    Ok(Some(show))
}

fn open_media(conn: &Connection, scid: i64) -> Result<Option<MediaItem>> {
    Ok(conn
        .query_row(
            "SELECT name, path, aRatio FROM media WHERE scid = ?1 LIMIT 1",
            params![scid],
            |row| {
                Ok(MediaItem {
                    short_path: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    long_path: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    aspect_ratio: row.get::<_, Option<i64>>(2)?.unwrap_or_default() as i32,
                })
            },
        )
        .optional()?)
}

fn open_announce(conn: &Connection, scid: i64) -> Result<Option<Announcement>> {
    let mut stmt = conn.prepare(
        "SELECT aId, title, aText, usePrivate, useAuto, loop, slideTimer, font, color, useBack, backPath, \
                alignV, alignH FROM announce WHERE scid = ?1 LIMIT 1",
    )?;
    let mut rows = stmt.query(params![scid])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Announcement {
            id: row.get::<_, Option<i64>>(0)?.unwrap_or_default(),
            title: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            text: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            use_private_settings: row.get::<_, Option<i64>>(3)?.unwrap_or_default() != 0,
            use_auto_next: row.get::<_, Option<i64>>(4)?.unwrap_or_default() != 0,
            loop_slides: row.get::<_, Option<i64>>(5)?.unwrap_or_default() != 0,
            slide_timer: row.get::<_, Option<i64>>(6)?.unwrap_or_default() as i32,
            font: font_from_row(row.get(7)?),
            color: color_from_row(row.get(8)?),
            use_background: row.get::<_, Option<i64>>(9)?.unwrap_or_default() != 0,
            background_path: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
            alignment_v: row.get::<_, Option<i64>>(11)?.unwrap_or_default() as i32,
            alignment_h: row.get::<_, Option<i64>>(12)?.unwrap_or_default() as i32,
        }))
    } else {
        Ok(None)
    }
}

fn text_or_int(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<String> {
    match row.get_ref(index)? {
        ValueRef::Null => Ok(String::new()),
        ValueRef::Integer(value) => Ok(value.to_string()),
        ValueRef::Real(value) => Ok(value.to_string()),
        ValueRef::Text(bytes) => Ok(String::from_utf8_lossy(bytes).to_string()),
        ValueRef::Blob(_) => Ok(String::new()),
    }
}

fn color_from_row(value: Option<i64>) -> RgbColor {
    RgbColor::from_qt(value.unwrap_or(0x00ff_ffff) as u32)
}

fn font_from_row(value: Option<String>) -> FontSpec {
    let text = value.unwrap_or_default();
    if text.trim().is_empty() {
        FontSpec::default()
    } else {
        FontSpec::from_qt(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.spsc");
        let mut items = vec![
            ScheduleItem::from_bible(BibleHistory {
                verse_ids: "1,2".into(),
                caption: "John 3:16-17".into(),
                caption_long: "John 3:16-17 (KJV)".into(),
            }),
            ScheduleItem::from_song(Song {
                title: "Amazing Grace".into(),
                number: 5,
                ..Default::default()
            }),
        ];
        save_schedule(&path, &mut items).unwrap();
        let loaded = open_schedule(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "John 3:16-17");
        assert!(matches!(loaded[1].kind, ScheduleKind::Song(_)));
    }
}
