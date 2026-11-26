//! Projection content and the renderer shared by the projector window and the
//! in-app preview.

use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, TextureHandle, TextureOptions, Ui, Vec2,
};
use sp_core::{FontSpec, RgbColor};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Visual style of a text slide.
#[derive(Clone, Debug)]
pub struct TextStyle {
    pub font: FontSpec,
    pub color: RgbColor,
    pub align_h: i32,
    pub align_v: i32,
    pub screen_use: i32,
    pub screen_position: i32,
    pub shadow: bool,
    pub caption_font: FontSpec,
    pub caption_color: RgbColor,
    pub caption_align_h: i32,
    pub caption_at_top: bool,
    pub info_font: FontSpec,
    pub info_color: RgbColor,
    pub info_at_top: bool,
    pub ending_font: FontSpec,
    pub ending_color: RgbColor,
    pub ending_at_top: bool,
    pub background: Option<Vec<u8>>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: FontSpec::new("Arial", 48),
            color: RgbColor::WHITE,
            align_h: 1,
            align_v: 1,
            screen_use: 100,
            screen_position: 1,
            shadow: true,
            caption_font: FontSpec::new("Arial", 36),
            caption_color: RgbColor::WHITE,
            caption_align_h: 1,
            caption_at_top: false,
            info_font: FontSpec::new("Arial", 36),
            info_color: RgbColor::WHITE,
            info_at_top: true,
            ending_font: FontSpec::new("Arial", 36),
            ending_color: RgbColor::WHITE,
            ending_at_top: false,
            background: None,
        }
    }
}

/// A text slide ready for projection.
#[derive(Clone, Debug, Default)]
pub struct TextSlide {
    pub text: String,
    pub caption: String,
    pub corner_left: String,
    pub corner_right: String,
    pub ending: String,
    pub style: TextStyle,
    pub label: String,
}

/// Anything that can be shown on the projector.
#[derive(Clone, Debug)]
pub enum Projected {
    Text(TextSlide),
    Image { bytes: Vec<u8>, label: String },
    Video { path: String, label: String },
    Blank { label: String },
}

impl Projected {
    pub fn label(&self) -> &str {
        match self {
            Projected::Text(slide) => &slide.label,
            Projected::Image { label, .. } => label,
            Projected::Video { label, .. } => label,
            Projected::Blank { label } => label,
        }
    }
}

/// The active projection queue.
#[derive(Clone, Debug, Default)]
pub struct Presentation {
    pub slides: Vec<Projected>,
    pub index: usize,
    pub source: String,
    pub black: bool,
    pub auto_next: bool,
    pub loop_slides: bool,
    pub slide_seconds: f32,
    /// -1.0 means "start the timer on the next frame".
    pub slide_shown_at: f64,
}

impl Presentation {
    pub fn new(source: impl Into<String>, slides: Vec<Projected>) -> Self {
        Self {
            slides,
            source: source.into(),
            index: 0,
            black: false,
            auto_next: false,
            loop_slides: false,
            slide_seconds: 0.0,
            slide_shown_at: -1.0,
        }
    }

    pub fn current(&self) -> Option<&Projected> {
        self.slides.get(self.index)
    }

    pub fn is_empty(&self) -> bool {
        self.slides.is_empty()
    }

    pub fn next(&mut self) -> bool {
        if self.index + 1 < self.slides.len() {
            self.index += 1;
            self.slide_shown_at = -1.0;
            true
        } else {
            false
        }
    }

    pub fn previous(&mut self) -> bool {
        if self.index > 0 {
            self.index -= 1;
            self.slide_shown_at = -1.0;
            true
        } else {
            false
        }
    }

    pub fn counter(&self) -> String {
        if self.slides.is_empty() {
            "0 / 0".to_string()
        } else {
            format!("{} / {}", self.index + 1, self.slides.len())
        }
    }
}

/// Cache of decoded image textures, keyed by a content hash.
#[derive(Default)]
pub struct ImageCache {
    textures: HashMap<u64, TextureHandle>,
}

impl ImageCache {
    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        namespace: &str,
        bytes: &[u8],
    ) -> Option<&TextureHandle> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        namespace.hash(&mut hasher);
        bytes.hash(&mut hasher);
        let key = hasher.finish();

        if !self.textures.contains_key(&key) {
            let image = image::load_from_memory(bytes).ok()?;
            let rgba = image.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            let texture = ctx.load_texture(
                format!("{namespace}-{key}"),
                color_image,
                TextureOptions::LINEAR,
            );
            self.textures.insert(key, texture);
        }
        self.textures.get(&key)
    }

    pub fn clear(&mut self) {
        self.textures.clear();
    }
}

pub fn color32(color: RgbColor) -> Color32 {
    Color32::from_rgb(color.r, color.g, color.b)
}

fn font_id(font: &FontSpec, scale: f32) -> FontId {
    let size = (font.point_size.max(6) as f32 * scale).clamp(6.0, 400.0);
    FontId::proportional(size)
}

fn anchor(align_h: i32, align_v: i32) -> Align2 {
    let x = match align_h {
        0 => egui::Align::Min,
        2 => egui::Align::Max,
        _ => egui::Align::Center,
    };
    let y = match align_v {
        0 => egui::Align::Min,
        2 => egui::Align::Max,
        _ => egui::Align::Center,
    };
    Align2([x, y])
}

fn draw_text(
    painter: &Painter,
    rect: Rect,
    text: &str,
    font: &FontSpec,
    color: RgbColor,
    align_h: i32,
    align_v: i32,
    shadow: bool,
    scale: f32,
) -> f32 {
    if text.trim().is_empty() || rect.width() <= 1.0 {
        return 0.0;
    }
    let font = font_id(font, scale);
    let galley = painter.layout(text.to_string(), font.clone(), color32(color), rect.width());
    let pos = anchor(align_h, align_v)
        .anchor_size(rect.center(), galley.size())
        .min;
    let pos = Pos2::new(
        pos.x.clamp(
            rect.left(),
            (rect.right() - galley.size().x).max(rect.left()),
        ),
        pos.y.clamp(
            rect.top(),
            (rect.bottom() - galley.size().y).max(rect.top()),
        ),
    );
    if shadow {
        let offset = (font.size / 15.0).max(1.0);
        painter.galley(
            pos + Vec2::new(offset, offset),
            galley.clone(),
            Color32::BLACK,
        );
    }
    let height = galley.size().y;
    painter.galley(pos, galley, color32(color));
    height
}

/// Shrink the text until it fits the given rectangle, then draw it.
fn draw_fitted_text(
    painter: &Painter,
    rect: Rect,
    text: &str,
    font: &FontSpec,
    color: RgbColor,
    align_h: i32,
    align_v: i32,
    shadow: bool,
    scale: f32,
    scale_down: bool,
) -> f32 {
    if text.trim().is_empty() || rect.height() <= 1.0 {
        return 0.0;
    }
    let mut point_size = font.point_size.max(8) as f32;
    let mut galley = {
        let mut probe = font.clone();
        probe.point_size = point_size as i32;
        painter.layout(
            text.to_string(),
            font_id(&probe, scale),
            color32(color),
            rect.width(),
        )
    };

    if scale_down {
        let min_size = (point_size * 0.35).max(10.0);
        while galley.size().y > rect.height() && point_size > min_size {
            point_size -= 1.0;
            let mut probe = font.clone();
            probe.point_size = point_size as i32;
            galley = painter.layout(
                text.to_string(),
                font_id(&probe, scale),
                color32(color),
                rect.width(),
            );
        }
    }

    let pos = anchor(align_h, align_v)
        .anchor_size(rect.center(), galley.size())
        .min;
    let pos = Pos2::new(
        pos.x.clamp(
            rect.left(),
            (rect.right() - galley.size().x).max(rect.left()),
        ),
        pos.y.clamp(
            rect.top(),
            (rect.bottom() - galley.size().y).max(rect.top()),
        ),
    );
    if shadow {
        let offset = (point_size * scale / 15.0).max(1.0);
        painter.galley(
            pos + Vec2::new(offset, offset),
            galley.clone(),
            Color32::BLACK,
        );
    }
    let height = galley.size().y;
    painter.galley(pos, galley, color32(color));
    height
}

/// Draw the current projection into `rect`.
pub fn render_projection(
    ui: &Ui,
    rect: Rect,
    projected: Option<&Projected>,
    black: bool,
    images: &mut ImageCache,
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::BLACK);

    if black {
        return;
    }

    let Some(projected) = projected else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "SoftProjector",
            FontId::proportional((rect.height() * 0.04).clamp(16.0, 80.0)),
            Color32::from_gray(80),
        );
        return;
    };

    match projected {
        Projected::Blank { .. } => {}
        Projected::Image { bytes, .. } => {
            if let Some(texture) = images.texture(ui.ctx(), "projected-image", bytes) {
                let size = texture.size_vec2();
                let scale = (rect.width() / size.x).min(rect.height() / size.y);
                let draw_size = size * scale;
                let image_rect = Rect::from_center_size(rect.center(), draw_size);
                painter.image(
                    texture.id(),
                    image_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        Projected::Video { path, .. } => {
            painter.text(
                rect.center() - Vec2::new(0.0, rect.height() * 0.03),
                Align2::CENTER_CENTER,
                "Video is playing in an external player",
                FontId::proportional((rect.height() * 0.035).clamp(14.0, 52.0)),
                Color32::from_gray(190),
            );
            painter.text(
                rect.center() + Vec2::new(0.0, rect.height() * 0.03),
                Align2::CENTER_CENTER,
                path,
                FontId::proportional((rect.height() * 0.022).clamp(10.0, 28.0)),
                Color32::from_gray(120),
            );
        }
        Projected::Text(slide) => draw_text_slide(ui, rect, slide, images),
    }
}

fn draw_text_slide(ui: &Ui, rect: Rect, slide: &TextSlide, images: &mut ImageCache) {
    let painter = ui.painter_at(rect);
    let style = &slide.style;
    let scale = (rect.height() / 1080.0).clamp(0.25, 3.0);

    // Background.
    if let Some(background) = &style.background {
        if let Some(texture) = images.texture(ui.ctx(), "theme-background", background) {
            painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }

    let margin_x = 30.0 * scale;
    let margin_y = 20.0 * scale;
    let content = rect.shrink2(Vec2::new(margin_x, margin_y));

    let mut top = content.top();
    let mut bottom = content.bottom();

    // Caption / info strip.
    let caption_text = if !slide.caption.is_empty() {
        Some((
            slide.caption.as_str(),
            &style.caption_font,
            style.caption_color,
            style.caption_at_top,
        ))
    } else {
        None
    };
    if let Some((caption, font, color, at_top)) = caption_text {
        let height =
            (font.point_size.max(8) as f32 * scale * 1.8).clamp(12.0, content.height() * 0.3);
        let strip = if at_top {
            let value = Rect::from_min_size(
                Pos2::new(content.left(), top),
                Vec2::new(content.width(), height),
            );
            top += height;
            value
        } else {
            let value = Rect::from_min_size(
                Pos2::new(content.left(), bottom - height),
                Vec2::new(content.width(), height),
            );
            bottom -= height;
            value
        };
        draw_text(
            &painter,
            strip,
            caption,
            font,
            color,
            style.caption_align_h,
            if at_top { 0 } else { 2 },
            style.shadow,
            scale,
        );
    }

    // Song info / corner strings.
    if !slide.corner_left.is_empty() || !slide.corner_right.is_empty() {
        let height = (style.info_font.point_size.max(8) as f32 * scale * 1.8)
            .clamp(12.0, content.height() * 0.3);
        let strip = if style.info_at_top {
            let value = Rect::from_min_size(
                Pos2::new(content.left(), top),
                Vec2::new(content.width(), height),
            );
            top += height;
            value
        } else {
            let value = Rect::from_min_size(
                Pos2::new(content.left(), bottom - height),
                Vec2::new(content.width(), height),
            );
            bottom -= height;
            value
        };
        let (left, right) = strip.split_left_right_at_fraction(0.7);
        draw_text(
            &painter,
            left,
            &slide.corner_left,
            &style.info_font,
            style.info_color,
            0,
            if style.info_at_top { 0 } else { 2 },
            style.shadow,
            scale,
        );
        draw_text(
            &painter,
            right,
            &slide.corner_right,
            &style.info_font,
            style.info_color,
            2,
            if style.info_at_top { 0 } else { 2 },
            style.shadow,
            scale,
        );
    }

    // Song ending.
    if !slide.ending.is_empty() {
        let height = (style.ending_font.point_size.max(8) as f32 * scale * 1.8)
            .clamp(12.0, content.height() * 0.3);
        let strip = if style.ending_at_top {
            let value = Rect::from_min_size(
                Pos2::new(content.left(), top),
                Vec2::new(content.width(), height),
            );
            top += height;
            value
        } else {
            let value = Rect::from_min_size(
                Pos2::new(content.left(), bottom - height),
                Vec2::new(content.width(), height),
            );
            bottom -= height;
            value
        };
        draw_text(
            &painter,
            strip,
            &slide.ending,
            &style.ending_font,
            style.ending_color,
            1,
            if style.ending_at_top { 0 } else { 2 },
            style.shadow,
            scale,
        );
    }

    let main_height = (bottom - top).max(10.0);
    let mut main_rect = Rect::from_min_size(
        Pos2::new(content.left(), top),
        Vec2::new(content.width(), main_height),
    );

    // Apply the `screenUse` / `screenPosition` theme settings, like the
    // original `displayScreen.cpp`.
    let screen_use = (style.screen_use.clamp(25, 100) as f32) / 100.0;
    let used_height = main_rect.height() * screen_use;
    if used_height < main_rect.height() {
        main_rect = if style.screen_position == 0 {
            Rect::from_min_size(main_rect.min, Vec2::new(main_rect.width(), used_height))
        } else {
            Rect::from_min_size(
                Pos2::new(main_rect.left(), main_rect.bottom() - used_height),
                Vec2::new(main_rect.width(), used_height),
            )
        };
    }

    draw_fitted_text(
        &painter,
        main_rect,
        &slide.text,
        &style.font,
        style.color,
        style.align_h,
        style.align_v,
        style.shadow,
        scale,
        true,
    );
}

/// Compose the ending string of a song, following `displayScreen.cpp`.
pub fn song_ending(theme_ending_type: i32, words_by: &str, music_by: &str) -> String {
    match theme_ending_type {
        0 => "*    *    *".into(),
        1 => "-    -    -".into(),
        2 => "♪    ♪    ♪".into(),
        3 => "♥    ♥    ♥".into(),
        4 => "✝    ✝    ✝".into(),
        5 => "☦    ☦    ☦".into(),
        6 => "❧    ❧    ❧".into(),
        7 => match (words_by.is_empty(), music_by.is_empty()) {
            (false, false) => format!("Words by: {words_by}, Music by: {music_by}"),
            (false, true) => format!("Words by: {words_by}"),
            (true, false) => format!("Music by: {music_by}"),
            (true, true) => "*    *    *".into(),
        },
        _ => "*    *    *".into(),
    }
}
