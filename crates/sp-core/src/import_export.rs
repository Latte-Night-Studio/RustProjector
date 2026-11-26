//! Compatibility layer for the import/export file formats of SoftProjector 2.
//!
//! * songbooks: legacy `##` text, `spSongBook` XML v2 and SQLite `.sps` v2
//! * Bibles: `##spDataVersion:\t1` text `.spb`
//! * themes: SQLite `.spt` v2

use crate::color::RgbColor;
use crate::database::Database;
use crate::error::{Result, SpError};
use crate::font::FontSpec;
use crate::song::{Song, Songbook, CATEGORIES};
use quick_xml::events::Event;
use quick_xml::Reader;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

fn to_multi_line(text: &str) -> String {
    text.replace("@%", "\n").trim().to_string()
}

fn clean_song_lines(text: &str) -> String {
    text.split("@$")
        .map(|verse| {
            verse
                .split("@%")
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        })
        .filter(|verse| !verse.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn parse_category(text: &str) -> i32 {
    let text = text.trim();
    if let Ok(value) = text.parse::<i32>() {
        return value;
    }
    CATEGORIES
        .iter()
        .position(|category| category.eq_ignore_ascii_case(text))
        .map(|index| index as i32)
        .unwrap_or(0)
}

fn parse_bool(text: &str) -> bool {
    matches!(
        text.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes"
    )
}

fn parse_color(text: &str) -> RgbColor {
    text.trim()
        .parse::<u32>()
        .map(RgbColor::from_qt)
        .unwrap_or(RgbColor::WHITE)
}

fn parse_alignment(text: &str) -> (i32, i32) {
    let mut parts = text.split(',');
    (
        parts
            .next()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(1),
        parts
            .next()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(1),
    )
}

/// Import a songbook.  The format is detected from the file contents.
/// Returns the id of the newly created songbook.
pub fn import_songbook(db: &Database, path: impl AsRef<Path>) -> Result<i64> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(b"SQLite format 3\0") {
        return import_songbook_sqlite(db, path);
    }

    let text = String::from_utf8_lossy(&bytes);
    let first = text.lines().next().unwrap_or_default();
    if first.trim_start().starts_with("<?xml") {
        import_songbook_xml(db, &text)
    } else if first.starts_with("##") {
        import_songbook_legacy(db, &text)
    } else {
        Err(SpError::InvalidData(
            "the file is not a supported songbook".into(),
        ))
    }
}

fn import_songbook_legacy(db: &Database, text: &str) -> Result<i64> {
    let normalized = text.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    let _magic = lines.next();
    let title = lines
        .next()
        .unwrap_or_default()
        .trim_start_matches('#')
        .trim()
        .to_string();
    let info = to_multi_line(
        lines
            .next()
            .unwrap_or_default()
            .trim_start_matches('#')
            .trim(),
    );
    let songbook_id = Songbook::add(db, &title, &info)?;
    let songbook_id_text = songbook_id.to_string();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split("#$#").collect();
        if parts.len() < 7 {
            continue;
        }
        let mut song = Song {
            songbook_id: songbook_id_text.clone(),
            songbook_name: title.clone(),
            number: parts[0].trim().parse().unwrap_or(1),
            title: parts[1].trim().to_string(),
            category: parse_category(parts[2]),
            tune: parts[3].trim().to_string(),
            words_by: parts[4].trim().to_string(),
            music_by: parts[5].trim().to_string(),
            song_text: clean_song_lines(parts[6]),
            ..Default::default()
        };
        if parts.len() > 7 {
            song.font = FontSpec::from_qt(parts[7]);
        }
        if parts.len() > 9 {
            song.background_name = parts[9].trim().to_string();
        }
        if parts.len() > 10 {
            song.notes = to_multi_line(parts[10]);
        }
        song.save_new(db)?;
    }
    Ok(songbook_id)
}

fn import_songbook_xml(db: &Database, text: &str) -> Result<i64> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);

    let mut songbook_title = String::new();
    let mut songbook_info = String::new();
    let mut songbook_id: i64 = 0;
    let mut current_song: Option<Song> = None;
    let mut current_element = String::new();
    let mut buffer = String::new();

    loop {
        match reader.read_event()? {
            Event::Start(event) => {
                let name = String::from_utf8_lossy(event.name().as_ref()).to_string();
                buffer.clear();
                match name.as_str() {
                    "spSongBook" => {
                        let mut supported = false;
                        for attribute in event.attributes().with_checks(false) {
                            let attribute = attribute?;
                            if attribute.key.as_ref() == b"version" {
                                let version = attribute.unescape_value()?.to_string();
                                supported = version.trim().parse::<f64>().unwrap_or(0.0) == 2.0;
                            }
                        }
                        if !supported {
                            return Err(SpError::InvalidData(
                                "unsupported songbook XML version (expected 2.0)".into(),
                            ));
                        }
                    }
                    "SongBook" => {
                        songbook_title.clear();
                        songbook_info.clear();
                    }
                    "Song" => {
                        let mut song = Song::default();
                        for attribute in event.attributes().with_checks(false) {
                            let attribute = attribute?;
                            if attribute.key.as_ref() == b"number" {
                                song.number =
                                    attribute.unescape_value()?.trim().parse().unwrap_or(1);
                            }
                        }
                        current_song = Some(song);
                    }
                    _ => {}
                }
                current_element = name;
            }
            Event::Empty(event) => {
                // Self-closing tags carry no useful data in the songbook format.
                let _ = event;
            }
            Event::Text(event) => {
                buffer.push_str(&event.unescape()?);
            }
            Event::CData(event) => {
                buffer.push_str(&String::from_utf8_lossy(event.as_ref()));
            }
            Event::End(event) => {
                let name = String::from_utf8_lossy(event.name().as_ref()).to_string();
                let value = buffer.trim().to_string();
                if let Some(song) = current_song.as_mut() {
                    match name.as_str() {
                        "title" => song.title = value,
                        "category" => song.category = parse_category(&value),
                        "tune" => song.tune = value,
                        "words" => song.words_by = value,
                        "music" => song.music_by = value,
                        "song_text" => {
                            song.song_text = if value.contains("@$") || value.contains("@%") {
                                clean_song_lines(&value)
                            } else {
                                value.clone()
                            }
                        }
                        "notes" => song.notes = value,
                        "use_private" => song.use_private_settings = parse_bool(&value),
                        "alignment" => {
                            let (v, h) = parse_alignment(&value);
                            song.alignment_v = v;
                            song.alignment_h = h;
                        }
                        "color" => song.color = parse_color(&value),
                        "font" => song.font = FontSpec::from_qt(&value),
                        "background" => song.background_name = value,
                        "count" => song.count = value.parse().unwrap_or(0),
                        "date" => song.date = value,
                        _ => {}
                    }
                } else {
                    match name.as_str() {
                        "title" => songbook_title = value,
                        "info" => songbook_info = value,
                        _ => {}
                    }
                }

                match name.as_str() {
                    "SongBook" => {
                        songbook_id = Songbook::add(db, &songbook_title, &songbook_info)?;
                    }
                    "Song" => {
                        if let Some(mut song) = current_song.take() {
                            song.songbook_id = songbook_id.to_string();
                            song.songbook_name = songbook_title.clone();
                            song.save_new(db)?;
                        }
                    }
                    _ => {}
                }
                buffer.clear();
                current_element.clear();
            }
            Event::Eof => break,
            _ => {}
        }
        let _ = &current_element;
    }

    if songbook_id == 0 {
        // An XML songbook without an explicit <SongBook> wrapper still gets a
        // useful default entry.
        songbook_id = Songbook::add(db, "Imported songbook", "")?;
    }
    Ok(songbook_id)
}

fn import_songbook_sqlite(db: &Database, path: &Path) -> Result<i64> {
    let source = Connection::open(path)?;
    let version: i64 = source.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version != 2 {
        return Err(SpError::DatabaseVersion {
            found: version,
            expected: 2,
        });
    }

    let (title, info): (String, String) = source
        .query_row("SELECT title, info FROM SongBook LIMIT 1", [], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            ))
        })
        .optional()?
        .unwrap_or_default();
    let songbook_id = Songbook::add(db, &title, &info)?;
    let songbook_id_text = songbook_id.to_string();

    let mut stmt = source.prepare(
        "SELECT number, title, category, tune, words, music, song_text, notes, use_private, alignment_v, \
                alignment_h, color, font, info_color, info_font, ending_color, ending_font, use_background, \
                background_name, background, count, date FROM Songs",
    )?;
    let rows = stmt.query_map([], |row| {
        let song_text = row.get::<_, Option<String>>(6)?.unwrap_or_default();
        let song_text = if song_text.contains("@$") || song_text.contains("@%") {
            clean_song_lines(&song_text)
        } else {
            song_text
        };
        Ok(Song {
            song_id: 0,
            songbook_id: songbook_id_text.clone(),
            songbook_name: title.clone(),
            number: row.get::<_, Option<i64>>(0)?.unwrap_or(1) as i32,
            title: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            category: row.get::<_, Option<i64>>(2)?.unwrap_or_default() as i32,
            tune: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            words_by: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            music_by: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            song_text,
            notes: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
            use_private_settings: row.get::<_, Option<i64>>(8)?.unwrap_or_default() != 0,
            alignment_v: row.get::<_, Option<i64>>(9)?.unwrap_or_default() as i32,
            alignment_h: row.get::<_, Option<i64>>(10)?.unwrap_or_default() as i32,
            color: RgbColor::from_qt(row.get::<_, Option<i64>>(11)?.unwrap_or(0x00ff_ffff) as u32),
            font: optional_font(row.get::<_, Option<String>>(12)?),
            info_color: RgbColor::from_qt(
                row.get::<_, Option<i64>>(13)?.unwrap_or(0x00ff_ffff) as u32
            ),
            info_font: optional_font(row.get::<_, Option<String>>(14)?),
            ending_color: RgbColor::from_qt(
                row.get::<_, Option<i64>>(15)?.unwrap_or(0x00ff_ffff) as u32
            ),
            ending_font: optional_font(row.get::<_, Option<String>>(16)?),
            use_background: row.get::<_, Option<i64>>(17)?.unwrap_or_default() != 0,
            background_name: row.get::<_, Option<String>>(18)?.unwrap_or_default(),
            background: row.get::<_, Option<Vec<u8>>>(19)?,
            count: row.get::<_, Option<i64>>(20)?.unwrap_or_default(),
            date: row.get::<_, Option<String>>(21)?.unwrap_or_default(),
        })
    })?;
    for row in rows {
        let mut song = row?;
        song.save_new(db)?;
    }
    Ok(songbook_id)
}

fn optional_font(value: Option<String>) -> FontSpec {
    let value = value.unwrap_or_default();
    if value.trim().is_empty() {
        FontSpec::default()
    } else {
        FontSpec::from_qt(&value)
    }
}

fn create_songbook_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA user_version = 2;
        CREATE TABLE 'SongBook' ('title' TEXT, 'info' TEXT);
        CREATE TABLE 'Songs' ('number' INTEGER, 'title' TEXT, 'category' INTEGER DEFAULT 0,
        'tune' TEXT, 'words' TEXT, 'music' TEXT, 'song_text' TEXT, 'notes' TEXT,
        'use_private' BOOL, 'alignment_v' INTEGER, 'alignment_h' INTEGER, 'color' INTEGER, 'font' TEXT,
        'info_color' INTEGER, 'info_font' TEXT, 'ending_color' INTEGER, 'ending_font' TEXT,
        'use_background' BOOL, 'background_name' TEXT, 'background' BLOB, 'count' INTEGER DEFAULT 0, 'date' TEXT);
        "#,
    )?;
    Ok(())
}

/// Export a songbook to a `.sps` SQLite file.
pub fn export_songbook(db: &Database, songbook_id: i64, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let target = Connection::open(path)?;
    create_songbook_schema(&target)?;

    let (title, info): (String, String) = db
        .conn()
        .query_row(
            "SELECT name, info FROM Songbooks WHERE id = ?1",
            params![songbook_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                ))
            },
        )
        .optional()?
        .ok_or_else(|| SpError::InvalidData("songbook not found".into()))?;
    target.execute(
        "INSERT INTO SongBook (title, info) VALUES (?1, ?2)",
        params![title, info],
    )?;

    let mut stmt = db.conn().prepare(
        "SELECT number, title, category, tune, words, music, song_text, notes, use_private, alignment_v, \
                alignment_h, color, font, info_color, info_font, ending_color, ending_font, use_background, \
                background_name, background, count, date FROM Songs WHERE songbook_id = ?1 ORDER BY number",
    )?;
    let mut rows = stmt.query(params![songbook_id.to_string()])?;
    while let Some(row) = rows.next()? {
        let values: Vec<rusqlite::types::Value> = (0..22)
            .map(|index| {
                row.get::<_, rusqlite::types::Value>(index)
                    .unwrap_or(rusqlite::types::Value::Null)
            })
            .collect();
        target.execute(
            "INSERT INTO Songs (number, title, category, tune, words, music, song_text, notes, use_private, \
              alignment_v, alignment_h, color, font, info_color, info_font, ending_color, ending_font, \
              use_background, background_name, background, count, date) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",
            rusqlite::params_from_iter(values),
        )?;
    }
    Ok(())
}

/// Import a SoftProjector Bible file (`.spb`, text version 1).
/// Returns the new `BibleVersions.id`.
pub fn import_bible(db: &Database, path: impl AsRef<Path>) -> Result<i64> {
    let text = std::fs::read_to_string(path)?;
    let normalized = text.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    let first = lines.next().unwrap_or_default();
    if !first.starts_with("##spData") {
        return Err(SpError::InvalidData(
            "unsupported Bible file (expected ##spData)".into(),
        ));
    }
    let version = first
        .split('\t')
        .nth(1)
        .map(|value| value.trim().to_string())
        .unwrap_or_default();
    if version != "1" {
        return Err(SpError::InvalidData(format!(
            "unsupported Bible file version {}",
            version
        )));
    }

    let mut title = String::new();
    let mut abbreviation = String::new();
    let mut information = String::new();
    let mut right_to_left = false;

    for _ in 0..4 {
        let line = lines.next().unwrap_or_default();
        let mut parts = line.split('\t');
        let key = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or_default().trim().to_string();
        match key {
            "##Title:" => title = value,
            "##Abbreviation:" => abbreviation = value,
            "##Information:" => information = to_multi_line(&value),
            "##RightToLeft:" => right_to_left = parse_bool(&value),
            _ => {}
        }
    }

    let mut books = Vec::new();
    for line in lines.by_ref() {
        if line.starts_with("---") {
            break;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            books.push((
                parts[0].trim().to_string(),
                parts[1].trim().to_string(),
                parts[2].trim().parse::<i64>().unwrap_or(0),
            ));
        }
    }

    let mut verses = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 5 {
            verses.push((
                parts[0].trim().to_string(),
                parts[1].trim().to_string(),
                parts[2].trim().parse::<i64>().unwrap_or(0),
                parts[3].trim().parse::<i64>().unwrap_or(0),
                parts[4].to_string(),
            ));
        }
    }

    let conn = db.conn();
    conn.execute(
        "INSERT INTO BibleVersions (bible_name, abbreviation, information, right_to_left) VALUES (?1,?2,?3,?4)",
        params![title, abbreviation, information, right_to_left],
    )?;
    let bible_id = conn.last_insert_rowid();
    let bible_id_text = bible_id.to_string();
    for (book_id, book_name, chapter_count) in books {
        conn.execute(
            "INSERT INTO BibleBooks (bible_id, id, book_name, chapter_count) VALUES (?1,?2,?3,?4)",
            params![bible_id_text, book_id, book_name, chapter_count],
        )?;
    }
    for (verse_id, book, chapter, verse, verse_text) in verses {
        conn.execute(
            "INSERT INTO BibleVerse (verse_id, bible_id, book, chapter, verse, verse_text) \
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![verse_id, bible_id_text, book, chapter, verse, verse_text],
        )?;
    }
    Ok(bible_id)
}

/// Export a Bible to the text `.spb` format.
pub fn export_bible(db: &Database, bible_id: i64, path: impl AsRef<Path>) -> Result<()> {
    let conn = db.conn();
    let (title, abbreviation, information, right_to_left): (String, String, String, i64) = conn
        .query_row(
            "SELECT bible_name, abbreviation, information, right_to_left FROM BibleVersions WHERE id = ?1",
            params![bible_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?.unwrap_or_default().trim().to_string(),
                    row.get::<_, Option<String>>(1)?.unwrap_or_default().trim().to_string(),
                    row.get::<_, Option<String>>(2)?.unwrap_or_default().trim().to_string(),
                    row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                ))
            },
        )
        .optional()?
        .ok_or_else(|| SpError::InvalidData("Bible version not found".into()))?;

    let info = information.replace("\n", "@%");
    let mut out = String::new();
    out.push_str("##spDataVersion:\t1\n");
    out.push_str(&format!("##Title:\t{}\n", title));
    out.push_str(&format!("##Abbreviation:\t{}\n", abbreviation));
    out.push_str(&format!("##Information:\t{}\n", info));
    out.push_str(&format!("##RightToLeft:\t{}\n", right_to_left));

    let mut stmt = conn.prepare(
        "SELECT id, book_name, chapter_count FROM BibleBooks WHERE bible_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map(params![bible_id.to_string()], |row| {
        Ok((
            row.get::<_, Option<i64>>(0)?.unwrap_or_default(),
            row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
        ))
    })?;
    for row in rows {
        let (id, name, chapters) = row?;
        out.push_str(&format!("{}\t{}\t{}\n", id, name.trim(), chapters));
    }
    out.push_str("-----");

    let mut stmt = conn.prepare(
        "SELECT verse_id, book, chapter, verse, verse_text FROM BibleVerse WHERE bible_id = ?1 \
         ORDER BY CAST(book AS INTEGER), chapter, verse",
    )?;
    let rows = stmt.query_map(params![bible_id.to_string()], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?.unwrap_or_default(),
            row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
            row.get::<_, Option<i64>>(3)?.unwrap_or_default(),
            row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        ))
    })?;
    for row in rows {
        let (verse_id, book, chapter, verse, text) = row?;
        out.push_str(&format!(
            "\n{}\t{}\t{}\t{}\t{}",
            verse_id.trim(),
            book.trim(),
            chapter,
            verse,
            text.replace('\n', " ")
        ));
    }
    std::fs::write(path, out)?;
    Ok(())
}

fn create_theme_export_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA user_version = 2;
        CREATE TABLE 'ThemeAnnounce' ('theme_id' INTEGER, 'disp' INTEGER, 'use_shadow' BOOL, 'use_fading' BOOL, 'use_blur_shadow' BOOL, 'use_background' BOOL, 'background_name' TEXT, 'background' BLOB, 'text_font' TEXT, 'text_color' INTEGER, 'text_align_v' INTEGER, 'text_align_h' INTEGER, 'use_disp_2' BOOL);
        CREATE TABLE 'ThemeBible' ('theme_id' INTEGER, 'disp' INTEGER, 'use_shadow' BOOL, 'use_fading' BOOL, 'use_blur_shadow' BOOL, 'use_background' BOOL, 'background_name' TEXT, 'background' BLOB, 'text_font' TEXT, 'text_color' INTEGER, 'text_align_v' INTEGER, 'text_align_h' INTEGER, 'caption_font' TEXT, 'caption_color' INTEGER, 'caption_align' INTEGER, 'caption_position' INTEGER, 'use_abbr' BOOL, 'screen_use' INTEGER, 'screen_position' INTEGER, 'use_disp_2' BOOL);
        CREATE TABLE 'ThemePassive' ('theme_id' INTEGER, 'disp' INTEGER, 'use_background' BOOL, 'background_name' TEXT, 'background' BLOB, 'use_disp_2' BOOL);
        CREATE TABLE 'ThemeSong' ('theme_id' INTEGER, 'disp' INTEGER, 'use_shadow' BOOL, 'use_fading' BOOL, 'use_blur_shadow' BOOL, 'show_stanza_title' BOOL, 'show_key' BOOL, 'show_number' BOOL, 'info_color' INTEGER, 'info_font' TEXT, 'info_align' INTEGER, 'show_song_ending' BOOL, 'ending_color' INTEGER, 'ending_font' TEXT, 'ending_type' INTEGER, 'ending_position' INTEGER, 'use_background' BOOL, 'background_name' TEXT, 'background' BLOB, 'text_font' TEXT, 'text_color' INTEGER, 'text_align_v' INTEGER, 'text_align_h' INTEGER, 'screen_use' INTEGER, 'screen_position' INTEGER, 'use_disp_2' BOOL);
        CREATE TABLE 'Themes' ('id' INTEGER , 'name' TEXT, 'comment' TEXT);
        "#,
    )?;
    Ok(())
}

/// Export one theme (`theme_id = Some(id)`) or all themes to a `.spt` file.
pub fn export_themes(db: &Database, path: impl AsRef<Path>, theme_id: Option<i64>) -> Result<()> {
    let path = path.as_ref();
    if db.path().to_string_lossy() == ":memory:" {
        return Err(SpError::InvalidData(
            "theme export requires a file-backed database".into(),
        ));
    }
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let target = Connection::open(path)?;
    create_theme_export_schema(&target)?;
    let source_path = db.path().to_string_lossy().to_string();
    target.execute("ATTACH DATABASE ?1 AS src", params![source_path])?;

    let ids: Vec<i64> = if let Some(id) = theme_id {
        vec![id]
    } else {
        let mut stmt = target.prepare("SELECT id FROM src.Themes ORDER BY id")?;
        let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        ids
    };

    for id in ids {
        target.execute(
            "INSERT INTO Themes (id, name, comment) SELECT id, name, comment FROM src.Themes WHERE id = ?1",
            params![id],
        )?;
        for table in ["ThemeAnnounce", "ThemeBible", "ThemePassive", "ThemeSong"] {
            target.execute(
                &format!(
                    "INSERT INTO {} SELECT * FROM src.{} WHERE theme_id = ?1",
                    table, table
                ),
                params![id],
            )?;
        }
    }
    target.execute("DETACH DATABASE src", [])?;
    Ok(())
}

/// Import themes from a `.spt` SQLite file.
pub fn import_themes(db: &Database, path: impl AsRef<Path>) -> Result<usize> {
    let path = path.as_ref();
    if db.path().to_string_lossy() == ":memory:" {
        return Err(SpError::InvalidData(
            "theme import requires a file-backed database".into(),
        ));
    }
    let source = Connection::open(path)?;
    let version: i64 = source.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version != 2 {
        return Err(SpError::DatabaseVersion {
            found: version,
            expected: 2,
        });
    }

    let mut stmt = source.prepare("SELECT id, name, comment FROM Themes ORDER BY id")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        ))
    })?;
    let themes = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    drop(stmt);

    source.execute(
        "ATTACH DATABASE ?1 AS dst",
        params![db.path().to_string_lossy()],
    )?;
    for (old_id, name, comment) in &themes {
        source.execute(
            "INSERT INTO dst.Themes (name, comment) VALUES (?1, ?2)",
            params![name, comment],
        )?;
        let new_id = source.last_insert_rowid();
        for table in ["ThemeAnnounce", "ThemeBible", "ThemePassive", "ThemeSong"] {
            let columns = table_columns(&source, table)?;
            let rest = columns
                .iter()
                .skip(1)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "INSERT INTO dst.{} (theme_id, {}) SELECT ?1, {} FROM main.{} WHERE theme_id = ?2",
                table, rest, rest, table
            );
            source.execute(&sql, params![new_id, old_id])?;
        }
    }
    source.execute("DETACH DATABASE dst", [])?;
    Ok(themes.len())
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info('{}')", table))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut columns = Vec::new();
    for row in rows {
        columns.push(row?);
    }
    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_songbook_import() {
        let db = Database::open_in_memory().unwrap();
        let mut file = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        write!(
            file,
            "##SoftProjector Songbook\n#Test Book\n#Info line\n1#$#First#$#Gospel#$#C#$#A#$#B#$#Verse 1@%line one@%line two@$Chorus@%refrain\n"
        )
        .unwrap();
        let id = import_songbook(&db, file.path()).unwrap();
        let songs = Song::load_all(&db).unwrap();
        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0].title, "First");
        assert!(songs[0].song_text.contains("\n\n"));
        assert_eq!(Songbook::load_all(&db).unwrap()[0].id, id);
    }

    #[test]
    fn bible_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("bible.spb");
        std::fs::write(
            &source,
            "##spDataVersion:\t1\n##Title:\tTest\n##Abbreviation:\tTST\n##Information:\tInfo\n##RightToLeft:\tfalse\n1\tJohn\t1\n-----\n1\t1\t1\t1\tIn the beginning\n",
        )
        .unwrap();
        let id = import_bible(&db, &source).unwrap();
        let exported = dir.path().join("out.spb");
        export_bible(&db, id, &exported).unwrap();
        let text = std::fs::read_to_string(exported).unwrap();
        assert!(text.contains("##Title:\tTest"));
        assert!(text.contains("In the beginning"));
    }
}
