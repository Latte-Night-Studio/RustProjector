use crate::database::Database;
use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisplayControlsSettings {
    /// 0=16, 1=24, 2=32, 3=48, 4=64, 5=96
    pub button_size: i32,
    pub alignment_v: i32,
    pub alignment_h: i32,
    pub opacity: f32,
}

impl Default for DisplayControlsSettings {
    fn default() -> Self {
        Self {
            button_size: 3,
            alignment_v: 2,
            alignment_h: 1,
            opacity: 0.3,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub display_is_on_top: bool,
    pub display_screen: i32,
    /// `-1` means "none".
    pub display_screen2: i32,
    pub current_theme_id: i64,
    pub display_controls: DisplayControlsSettings,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            display_is_on_top: false,
            display_screen: 0,
            display_screen2: -1,
            current_theme_id: 0,
            display_controls: DisplayControlsSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpMainSettings {
    pub sp_splitter: Vec<u8>,
    pub bible_hidden_splitter: Vec<u8>,
    pub bible_show_splitter: Vec<u8>,
    pub song_splitter: Vec<u8>,
    pub ui_translation: String,
    pub is_window_maximized: bool,
}

impl SpMainSettings {
    pub fn with_defaults() -> Self {
        Self {
            sp_splitter: Vec::new(),
            bible_hidden_splitter: Vec::new(),
            bible_show_splitter: Vec::new(),
            song_splitter: Vec::new(),
            ui_translation: "en".into(),
            is_window_maximized: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BibleVersionSettings {
    pub primary_bible: String,
    pub secondary_bible: String,
    pub trinary_bible: String,
    pub operator_bible: String,
}

impl Default for BibleVersionSettings {
    fn default() -> Self {
        Self {
            primary_bible: "none".into(),
            secondary_bible: "none".into(),
            trinary_bible: "none".into(),
            operator_bible: "same".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlideShowSettings {
    pub expand_small: bool,
    pub fit_type: i32,
    pub resize: bool,
    pub bound_type: i32,
    pub bound_width: i32,
}

impl Default for SlideShowSettings {
    fn default() -> Self {
        Self {
            expand_small: true,
            fit_type: 0,
            resize: true,
            bound_type: 2,
            bound_width: 1280,
        }
    }
}

/// All settings persisted in the `Settings` table.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    pub general: GeneralSettings,
    pub sp_main: SpMainSettings,
    pub bible_sets: BibleVersionSettings,
    pub bible_sets2: BibleVersionSettings,
    pub slide_sets: SlideShowSettings,
}

impl Settings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load settings from the `Settings` table, creating defaults for any
    /// missing section just like the Qt application does.
    pub fn load(db: &Database) -> Result<Self> {
        let mut settings = Settings::default();
        let mut found_general = false;
        let mut found_sp = false;
        let mut found_b1 = false;
        let mut found_b2 = false;
        let mut found_pix = false;

        let mut stmt = db.conn().prepare("SELECT type, sets FROM Settings")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let kind: String = row.get::<_, Option<String>>(0)?.unwrap_or_default();
            let blob: String = row.get::<_, Option<String>>(1)?.unwrap_or_default();
            match kind.as_str() {
                "general" => {
                    found_general = true;
                    settings.general = parse_general(&blob);
                }
                "spMain" => {
                    found_sp = true;
                    settings.sp_main = parse_sp_main(&blob);
                }
                "bible1" => {
                    found_b1 = true;
                    settings.bible_sets = parse_bible(&blob, true);
                }
                "bible2" => {
                    found_b2 = true;
                    settings.bible_sets2 = parse_bible(&blob, false);
                }
                "pix" => {
                    found_pix = true;
                    settings.slide_sets = parse_pix(&blob);
                }
                _ => {}
            }
        }

        if !(found_general && found_sp && found_b1 && found_b2 && found_pix) {
            settings.save_new(db)?;
        }
        Ok(settings)
    }

    pub fn save(&self, db: &Database) -> Result<()> {
        db.set_setting_blob("general", &general_to_string(&self.general))?;
        db.set_setting_blob("spMain", &sp_main_to_string(&self.sp_main))?;
        db.set_setting_blob("bible1", &bible_to_string(&self.bible_sets, true))?;
        db.set_setting_blob("bible2", &bible_to_string(&self.bible_sets2, false))?;
        db.set_setting_blob("pix", &pix_to_string(&self.slide_sets))?;
        Ok(())
    }

    /// Write default settings after a section was missing.
    fn save_new(&self, db: &Database) -> Result<()> {
        let defaults = Settings::default();
        defaults.save(db)
    }
}

fn parse_pairs(blob: &str) -> Vec<(String, String)> {
    blob.replace("\r\n", "\n")
        .split('\n')
        .filter_map(|line| {
            let (name, value) = line.split_once('=')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn parse_bool(value: &str) -> bool {
    value.eq_ignore_ascii_case("true") || value == "1"
}

fn parse_general(blob: &str) -> GeneralSettings {
    let mut out = GeneralSettings::default();
    for (name, value) in parse_pairs(blob) {
        match name.as_str() {
            "displayIsOnTop" => out.display_is_on_top = parse_bool(&value),
            "currentThemeId" => out.current_theme_id = value.parse().unwrap_or_default(),
            "displayScreen" => out.display_screen = value.parse().unwrap_or_default(),
            "displayScreen2" => out.display_screen2 = value.parse().unwrap_or(-1),
            "dcIconSize" => out.display_controls.button_size = value.parse().unwrap_or(3),
            "dcAlignment" => {
                let mut parts = value.split(',');
                out.display_controls.alignment_v =
                    parts.next().and_then(|v| v.parse().ok()).unwrap_or(2);
                out.display_controls.alignment_h =
                    parts.next().and_then(|v| v.parse().ok()).unwrap_or(1);
            }
            "dcOpacity" => out.display_controls.opacity = value.parse().unwrap_or(0.3),
            _ => {}
        }
    }
    out
}

fn parse_sp_main(blob: &str) -> SpMainSettings {
    let mut out = SpMainSettings::with_defaults();
    for (name, value) in parse_pairs(blob) {
        match name.as_str() {
            "spSplitter" => out.sp_splitter = decode_hex(&value),
            "bibleHiddenSplitter" => out.bible_hidden_splitter = decode_hex(&value),
            "bibleShowSplitter" => out.bible_show_splitter = decode_hex(&value),
            "songSplitter" => out.song_splitter = decode_hex(&value),
            "uiTranslation" => out.ui_translation = value,
            "isWindowMaximized" => out.is_window_maximized = parse_bool(&value),
            _ => {}
        }
    }
    out
}

fn parse_bible(blob: &str, with_operator: bool) -> BibleVersionSettings {
    let mut out = BibleVersionSettings::default();
    for (name, value) in parse_pairs(blob) {
        match name.as_str() {
            "primary" => out.primary_bible = value,
            "secondary" => out.secondary_bible = value,
            "trinary" => out.trinary_bible = value,
            "operator" if with_operator => out.operator_bible = value,
            _ => {}
        }
    }
    out
}

fn parse_pix(blob: &str) -> SlideShowSettings {
    let mut out = SlideShowSettings::default();
    for (name, value) in parse_pairs(blob) {
        match name.as_str() {
            "expandSmall" => out.expand_small = parse_bool(&value),
            "fitType" => out.fit_type = value.parse().unwrap_or(0),
            "resize" => out.resize = parse_bool(&value),
            "boundType" => out.bound_type = value.parse().unwrap_or(2),
            "boundWidth" => out.bound_width = value.parse().unwrap_or(1280),
            _ => {}
        }
    }
    out
}

fn general_to_string(g: &GeneralSettings) -> String {
    format!(
        "displayIsOnTop = {}\ncurrentThemeId = {}\ndisplayScreen = {}\ndisplayScreen2 = {}\ndcIconSize = {}\ndcAlignment = {},{}\ndcOpacity = {}",
        g.display_is_on_top,
        g.current_theme_id,
        g.display_screen,
        g.display_screen2,
        g.display_controls.button_size,
        g.display_controls.alignment_v,
        g.display_controls.alignment_h,
        g.display_controls.opacity
    )
}

fn sp_main_to_string(s: &SpMainSettings) -> String {
    format!(
        "spSplitter = {}\nbibleHiddenSplitter = {}\nbibleShowSplitter = {}\nsongSplitter = {}\nuiTranslation = {}\nisWindowMaximized = {}",
        encode_hex(&s.sp_splitter),
        encode_hex(&s.bible_hidden_splitter),
        encode_hex(&s.bible_show_splitter),
        encode_hex(&s.song_splitter),
        s.ui_translation,
        s.is_window_maximized
    )
}

fn bible_to_string(b: &BibleVersionSettings, with_operator: bool) -> String {
    let mut out = format!(
        "primary = {}\nsecondary = {}\ntrinary = {}",
        b.primary_bible, b.secondary_bible, b.trinary_bible
    );
    if with_operator {
        out.push_str(&format!("\noperator = {}", b.operator_bible));
    }
    out
}

fn pix_to_string(p: &SlideShowSettings) -> String {
    format!(
        "expandSmall = {}\nfitType = {}\nresize = {}\nboundType = {}\nboundWidth = {}",
        p.expand_small, p.fit_type, p.resize, p.bound_type, p.bound_width
    )
}

fn decode_hex(value: &str) -> Vec<u8> {
    let value = value.trim();
    if value.len() % 2 != 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let pair = &value[i..i + 2];
        match u8::from_str_radix(pair, 16) {
            Ok(byte) => out.push(byte),
            Err(_) => return Vec::new(),
        }
        i += 2;
    }
    out
}

fn encode_hex(value: &[u8]) -> String {
    let mut out = String::with_capacity(value.len() * 2);
    for byte in value {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut s = Settings::default();
        s.general.current_theme_id = 7;
        s.general.display_controls.opacity = 0.75;
        s.bible_sets.primary_bible = "1".into();
        s.slide_sets.bound_width = 1920;

        assert_eq!(
            parse_general(&general_to_string(&s.general)).current_theme_id,
            7
        );
        assert_eq!(
            parse_bible(&bible_to_string(&s.bible_sets, true), true).primary_bible,
            "1"
        );
        assert_eq!(parse_pix(&pix_to_string(&s.slide_sets)).bound_width, 1920);
    }

    #[test]
    fn hex_roundtrip() {
        let data = vec![0, 1, 15, 16, 255];
        assert_eq!(decode_hex(&encode_hex(&data)), data);
    }
}
