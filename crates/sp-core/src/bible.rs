use crate::database::Database;
use crate::error::Result;
use crate::settings::BibleVersionSettings;
use crate::theme::BibleSettings;
use regex::RegexBuilder;
use rusqlite::params;

#[derive(Clone, Debug, Default)]
pub struct BibleVersion {
    pub id: i64,
    pub name: String,
    pub abbreviation: String,
    pub information: String,
    pub right_to_left: bool,
}

impl BibleVersion {
    pub fn load_all(db: &Database) -> Result<Vec<BibleVersion>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, bible_name, abbreviation, information, right_to_left \
             FROM BibleVersions ORDER BY bible_name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BibleVersion {
                id: row.get(0)?,
                name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                abbreviation: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                information: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                right_to_left: row.get::<_, Option<i64>>(4)?.unwrap_or(0) != 0,
            })
        })?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub fn load(db: &Database, id: i64) -> Result<Option<BibleVersion>> {
        Ok(Self::load_all(db)?.into_iter().find(|v| v.id == id))
    }

    pub fn delete(db: &Database, id: i64) -> Result<()> {
        let conn = db.conn();
        conn.execute("DELETE FROM BibleVersions WHERE id = ?1", params![id])?;
        conn.execute(
            "DELETE FROM BibleBooks WHERE bible_id = ?1",
            params![id.to_string()],
        )?;
        conn.execute(
            "DELETE FROM BibleVerse WHERE bible_id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct BibleBook {
    pub book: String,
    pub book_id: i32,
    pub chapter_count: i32,
}

#[derive(Clone, Debug, Default)]
pub struct BibleVerse {
    pub verse_id: String,
    pub book: i32,
    pub chapter: i32,
    pub verse_number: i32,
    pub verse_text: String,
}

#[derive(Clone, Debug, Default)]
pub struct BibleSearch {
    pub book: String,
    pub chapter: String,
    pub verse: String,
    pub verse_text: String,
    pub verse_id: String,
    pub display_text: String,
    pub first_v: i32,
    pub last_v: i32,
}

#[derive(Clone, Debug, Default)]
pub struct BibleHistory {
    pub verse_ids: String,
    pub caption: String,
    pub caption_long: String,
}

/// Text and captions for one or more Bible versions at the same time.
#[derive(Clone, Debug, Default)]
pub struct Verse {
    pub primary_text: String,
    pub secondary_text: String,
    pub primary_caption: String,
    pub secondary_caption: String,
    pub trinary_text: String,
    pub trinary_caption: String,
}

/// Loaded Bible used for browsing and searching.
#[derive(Clone, Debug, Default)]
pub struct BibleSession {
    pub version: BibleVersion,
    pub books: Vec<BibleBook>,
    pub verses: Vec<BibleVerse>,
    pub preview_id_list: Vec<String>,
    pub current_id_list: Vec<String>,
}

impl BibleSession {
    /// Load a complete Bible version into memory, ready for browsing.
    pub fn load(db: &Database, version_id: i64) -> Result<Option<BibleSession>> {
        let Some(version) = BibleVersion::load(db, version_id)? else {
            return Ok(None);
        };
        let conn = db.conn();
        let mut books = Vec::new();
        {
            let mut stmt = conn.prepare(
                "SELECT id, book_name, chapter_count FROM BibleBooks WHERE bible_id = ?1 ORDER BY id",
            )?;
            let rows = stmt.query_map(params![version_id.to_string()], |row| {
                Ok(BibleBook {
                    book_id: row.get::<_, Option<i64>>(0)?.unwrap_or(0) as i32,
                    book: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    chapter_count: row.get::<_, Option<i64>>(2)?.unwrap_or(0) as i32,
                })
            })?;
            for row in rows {
                books.push(row?);
            }
        }
        let mut verses = Vec::new();
        {
            let mut stmt = conn.prepare(
                "SELECT verse_id, book, chapter, verse, verse_text FROM BibleVerse WHERE bible_id = ?1 \
                 ORDER BY CAST(book AS INTEGER), chapter, verse",
            )?;
            let rows = stmt.query_map(params![version_id.to_string()], |row| {
                Ok(BibleVerse {
                    verse_id: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    book: row
                        .get::<_, Option<String>>(1)?
                        .unwrap_or_default()
                        .trim()
                        .parse()
                        .unwrap_or(0),
                    chapter: row.get::<_, Option<i64>>(2)?.unwrap_or(0) as i32,
                    verse_number: row.get::<_, Option<i64>>(3)?.unwrap_or(0) as i32,
                    verse_text: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                })
            })?;
            for row in rows {
                verses.push(row?);
            }
        }
        Ok(Some(BibleSession {
            version,
            books,
            verses,
            preview_id_list: Vec::new(),
            current_id_list: Vec::new(),
        }))
    }

    pub fn books(&self) -> Vec<String> {
        self.books.iter().map(|b| b.book.clone()).collect()
    }

    pub fn book_name(&self, id: i32) -> String {
        self.books
            .iter()
            .find(|b| b.book_id == id)
            .map(|b| b.book.clone())
            .unwrap_or_default()
    }

    pub fn book_id(&self, name: &str) -> Option<i32> {
        self.books
            .iter()
            .find(|b| b.book == name)
            .map(|b| b.book_id)
    }

    pub fn book_row(&self, name: &str) -> Option<usize> {
        self.books.iter().position(|b| b.book == name)
    }

    /// Return the verses of a chapter as display strings.  Rows with the same
    /// verse number are merged, as in the original `Bible::getChapter`.
    pub fn chapter(&mut self, book: i32, chapter: i32) -> Vec<String> {
        let mut list = Vec::new();
        self.preview_id_list.clear();

        let mut verse_text = String::new();
        let mut id = String::new();
        let mut verse_old = 0;
        for bv in self
            .verses
            .iter()
            .filter(|v| v.book == book && v.chapter == chapter)
        {
            if bv.verse_number == verse_old {
                verse_text = format!(
                    "{} {}",
                    verse_text.split_whitespace().collect::<Vec<_>>().join(" "),
                    bv.verse_text
                );
                id.push(',');
                id.push_str(&bv.verse_id);
                list.pop();
                self.preview_id_list.pop();
            } else {
                verse_text = bv.verse_text.clone();
                id = bv.verse_id.clone();
            }
            list.push(format!("{}. {}", bv.verse_number, verse_text));
            self.preview_id_list.push(id.clone());
            verse_old = bv.verse_number;
        }
        list
    }

    pub fn verse_ref(&self, verse_id: &str) -> Option<(String, i32, i32)> {
        let id = verse_id.split(',').next().unwrap_or(verse_id);
        let bv = self.verses.iter().find(|v| v.verse_id == id)?;
        Some((self.book_name(bv.book), bv.chapter, bv.verse_number))
    }

    pub fn last_verse_number(&self, verse_id: &str) -> i32 {
        let id = verse_id.split(',').next_back().unwrap_or(verse_id);
        self.verses
            .iter()
            .find(|v| v.verse_id == id)
            .map(|v| v.verse_number)
            .unwrap_or(0)
    }

    /// Search the loaded Bible.  `pattern` uses the same convention as the
    /// original application: spaces in the user query become `\\W*`.
    pub fn search(
        &self,
        all_words: bool,
        pattern: &str,
        book: Option<i32>,
        chapter: Option<i32>,
    ) -> Vec<BibleSearch> {
        let re = RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .or_else(|_| {
                RegexBuilder::new(&regex::escape(pattern))
                    .case_insensitive(true)
                    .build()
            })
            .ok();

        let Some(re) = re else { return Vec::new() };

        let mut words = Vec::new();
        if all_words {
            let stripped = pattern
                .replace("\\b(", "")
                .replace(")\\b", "")
                .replace('(', "")
                .replace(')', "");
            for word in stripped.split('|') {
                if word.is_empty() {
                    continue;
                }
                if let Ok(w) = RegexBuilder::new(&format!("\\b{}\\b", regex::escape(word)))
                    .case_insensitive(true)
                    .build()
                {
                    words.push(w);
                }
            }
        }

        let mut results = Vec::new();
        for bv in &self.verses {
            if let Some(b) = book {
                if bv.book != b {
                    continue;
                }
            }
            if let Some(c) = chapter {
                if bv.chapter != c {
                    continue;
                }
            }
            if !re.is_match(&bv.verse_text) {
                continue;
            }
            if all_words && !words.iter().all(|w| w.is_match(&bv.verse_text)) {
                continue;
            }

            let book_name = self.book_name(bv.book);
            results.push(BibleSearch {
                book: book_name.clone(),
                chapter: bv.chapter.to_string(),
                verse: bv.verse_number.to_string(),
                verse_text: bv.verse_text.clone(),
                verse_id: bv.verse_id.clone(),
                display_text: format!(
                    "{} {}:{} {}",
                    book_name, bv.chapter, bv.verse_number, bv.verse_text
                ),
                first_v: bv.verse_number,
                last_v: bv.verse_number,
            });
        }
        results
    }
}

/// Build the text and caption for one Bible version from a comma separated list
/// of verse ids.  This is a port of `Bible::getVerseAndCaption`.
pub fn build_verse_and_caption(
    verses: &[BibleVerse],
    books: &[BibleBook],
    abbreviation: &str,
    verse_ids: &str,
    use_abbr: bool,
) -> (String, String) {
    let ids: Vec<&str> = verse_ids.split(',').filter(|s| !s.is_empty()).collect();
    if ids.is_empty() {
        return (String::new(), String::new());
    }

    let mut selected: Vec<&BibleVerse> = Vec::new();
    for id in &ids {
        for v in verses {
            if v.verse_id == *id {
                selected.push(v);
            }
        }
    }
    if selected.is_empty() {
        return (String::new(), String::new());
    }

    let book = selected[0].book;
    let chapter = selected[0].chapter;
    let mut verse_nfirst = selected[0].verse_number;
    let mut verse_show = String::new();
    let mut caption = String::new();

    if !verse_ids.contains(',') {
        // Single verse: no verse number in the text, simple `chapter:verse`
        // caption.
        verse_show = selected[0].verse_text.trim().to_string();
        caption = format!("{}:{}", chapter, verse_nfirst);
    } else {
        let mut verse_nold: Option<i32> = None;
        let mut first = true;
        for bv in selected {
            let verse_n = bv.verse_number;
            if first {
                verse_nfirst = verse_n;
                first = false;
            }

            if Some(verse_n) == verse_nold {
                if verse_n == verse_nfirst {
                    caption = format!("{}:{}", chapter, verse_n);
                    // Remove the verse-number prefix inserted for the first row
                    // and append the remaining text of the same verse.
                    if let Some(pos) = verse_show.find(')') {
                        verse_show = verse_show[pos + 1..].trim_start().to_string();
                    }
                } else {
                    caption = format!("{}:{}-{}", chapter, verse_nfirst, verse_n);
                }
                verse_show.push(' ');
                verse_show.push_str(bv.verse_text.trim());
            } else {
                caption = format!("{}:{}-{}", chapter, verse_nfirst, verse_n);
                verse_show.push_str(&format!(" ({}) {}", verse_n, bv.verse_text.trim()));
                if !verse_show.starts_with(" (") {
                    verse_show = format!(" ({}) {}", verse_nfirst, verse_show);
                }
            }
            verse_nold = Some(verse_n);
        }
    }

    let book_name = books
        .iter()
        .find(|b| b.book_id == book)
        .map(|b| b.book.clone())
        .unwrap_or_default();

    let mut caption_full = format!("{} {}", book_name, caption);
    if use_abbr && !abbreviation.trim().is_empty() {
        caption_full = format!("{} ({})", caption_full, abbreviation.trim());
    }

    (
        verse_show.split_whitespace().collect::<Vec<_>>().join(" "),
        caption_full
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Compose up to three Bible versions into the display structure used by the
/// projection window.
pub fn compose_verse(
    primary: Option<&BibleSession>,
    secondary: Option<&BibleSession>,
    trinary: Option<&BibleSession>,
    verse_ids: &str,
    sets: &BibleSettings,
    versions: &BibleVersionSettings,
) -> Verse {
    let mut out = Verse::default();

    if let Some(session) = primary {
        let (text, caption) = build_verse_and_caption(
            &session.verses,
            &session.books,
            &session.version.abbreviation,
            verse_ids,
            sets.use_abbriviations,
        );
        out.primary_text = text;
        out.primary_caption = caption;
    }
    if versions.secondary_bible != versions.primary_bible && versions.secondary_bible != "none" {
        if let Some(session) = secondary {
            let (text, caption) = build_verse_and_caption(
                &session.verses,
                &session.books,
                &session.version.abbreviation,
                verse_ids,
                sets.use_abbriviations,
            );
            out.secondary_text = text;
            out.secondary_caption = caption;
        }
    }
    if versions.trinary_bible != versions.primary_bible
        && versions.trinary_bible != versions.secondary_bible
        && versions.trinary_bible != "none"
    {
        if let Some(session) = trinary {
            let (text, caption) = build_verse_and_caption(
                &session.verses,
                &session.books,
                &session.version.abbreviation,
                verse_ids,
                sets.use_abbriviations,
            );
            out.trinary_text = text;
            out.trinary_caption = caption;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_verse_rows() {
        let books = vec![BibleBook {
            book: "John".into(),
            book_id: 43,
            chapter_count: 21,
        }];
        let verses = vec![
            BibleVerse {
                verse_id: "1".into(),
                book: 43,
                chapter: 3,
                verse_number: 16,
                verse_text: "For God".into(),
            },
            BibleVerse {
                verse_id: "2".into(),
                book: 43,
                chapter: 3,
                verse_number: 16,
                verse_text: "so loved".into(),
            },
        ];
        let (text, caption) = build_verse_and_caption(&verses, &books, "KJV", "1,2", false);
        assert_eq!(text, "For God so loved");
        assert_eq!(caption, "John 3:16");
    }
}
