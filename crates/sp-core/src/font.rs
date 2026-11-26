use serde::{Deserialize, Serialize};

/// Font description stored in SoftProjector databases.
///
/// Qt serialises `QFont` as a comma separated string such as
/// `Arial,48,-1,5,50,0,0,0,0,0`:
/// family, point size, pixel size, style hint, weight, style, underline,
/// strike out, fixed pitch, raw mode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontSpec {
    pub family: String,
    pub point_size: i32,
    pub weight: i32,
    pub italic: bool,
    pub underline: bool,
    pub strike_out: bool,
}

impl Default for FontSpec {
    fn default() -> Self {
        Self::new("Arial", 48)
    }
}

impl FontSpec {
    pub fn new(family: impl Into<String>, point_size: i32) -> Self {
        Self {
            family: family.into(),
            point_size,
            weight: 50,
            italic: false,
            underline: false,
            strike_out: false,
        }
    }

    pub fn from_qt(value: &str) -> Self {
        let mut parts = value.split(',');
        let family = parts.next().unwrap_or("Arial").trim();
        let point_size = parts
            .next()
            .and_then(|v| v.trim().parse::<i32>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(12);
        let _pixel_size = parts.next();
        let _style_hint = parts.next();
        let weight = parts
            .next()
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(50);
        let style = parts
            .next()
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0);
        let underline = parts
            .next()
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0)
            != 0;
        let strike_out = parts
            .next()
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0)
            != 0;

        Self {
            family: if family.is_empty() {
                "Arial".into()
            } else {
                family.into()
            },
            point_size,
            weight,
            italic: style == 1 || style == 2,
            underline,
            strike_out,
        }
    }

    pub fn to_qt(&self) -> String {
        format!(
            "{},{},-1,5,{}, {}, {}, {},0,0",
            self.family,
            self.point_size,
            self.weight,
            if self.italic { 1 } else { 0 },
            if self.underline { 1 } else { 0 },
            if self.strike_out { 1 } else { 0 }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qt_roundtrip() {
        let font = FontSpec::from_qt("Arial,48,-1,5,50,0,0,0,0,0");
        assert_eq!(font.family, "Arial");
        assert_eq!(font.point_size, 48);
        assert!(!font.italic);
        assert_eq!(FontSpec::from_qt(&font.to_qt()), font);
    }
}
