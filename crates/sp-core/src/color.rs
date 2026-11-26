use serde::{Deserialize, Serialize};

/// An sRGB colour.  SoftProjector stores colours as Qt `QRgb` values
/// (`0xAARRGGBB`) in SQLite; this type keeps the RGB part and converts back to
/// the same representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Default for RgbColor {
    fn default() -> Self {
        Self::WHITE
    }
}

impl RgbColor {
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Decode a Qt `QRgb`/`QColor::rgb()` value (alpha is ignored).
    pub fn from_qt(value: u32) -> Self {
        Self {
            r: ((value >> 16) & 0xff) as u8,
            g: ((value >> 8) & 0xff) as u8,
            b: (value & 0xff) as u8,
        }
    }

    /// Encode as a Qt `QRgb` value with an opaque alpha channel.
    pub fn to_qt(self) -> u32 {
        0xff00_0000 | ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }

    pub fn to_array(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }

    pub fn from_array(rgb: [u8; 3]) -> Self {
        Self {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
        }
    }
}
