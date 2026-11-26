//! Theme list and per-display theme editor.

use crate::app::{App, ThemeSection};
use crate::widgets;
use eframe::egui::{self, RichText};
use sp_core::import_export::{export_themes, import_themes};
use sp_core::theme::{AnnounceSettings, BibleSettings, PassiveSettings, SongSettings, Theme};

impl App {
    pub fn themes_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Themes");
            if ui.button("➕ New").clicked() {
                let mut theme = Theme::default();
                theme.name = "New theme".into();
                theme.comments.clear();
                match theme.save_new(&self.db) {
                    Ok(()) => {
                        self.reload_themes();
                        self.load_theme(theme.theme_id);
                        self.set_status("Theme created");
                    }
                    Err(err) => self.set_error("Could not create theme", err),
                }
            }
            if ui.button("⧉ Duplicate").clicked() {
                let name = format!("{} copy", self.theme.name);
                match self.theme.duplicate(&self.db, &name, &self.theme.comments) {
                    Ok(id) => {
                        self.reload_themes();
                        self.load_theme(id);
                        self.set_status("Theme duplicated");
                    }
                    Err(err) => self.set_error("Could not duplicate theme", err),
                }
            }
            if ui.button("🗑 Delete").clicked() {
                if self.theme_infos.len() <= 1 {
                    self.set_error("Could not delete theme", "at least one theme is required");
                } else {
                    let id = self.theme.theme_id;
                    match Theme::delete(&self.db, id) {
                        Ok(()) => {
                            self.reload_themes();
                            let next = self.theme_infos.first().map(|t| t.theme_id).unwrap_or(0);
                            self.load_theme(next);
                            self.set_status("Theme deleted");
                        }
                        Err(err) => self.set_error("Could not delete theme", err),
                    }
                }
            }
            ui.separator();
            if ui.button("📥 Import themes...").clicked() {
                self.import_themes_dialog();
            }
            if ui.button("📤 Export all themes...").clicked() {
                self.export_themes_dialog();
            }
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Editing display:");
            ui.selectable_value(&mut self.theme_display, 1, "Display 1");
            ui.selectable_value(&mut self.theme_display, 2, "Display 2");
            ui.separator();
            for (section, label) in [
                (ThemeSection::Bible, "Bible"),
                (ThemeSection::Song, "Songs"),
                (ThemeSection::Announce, "Announcements"),
                (ThemeSection::Passive, "Passive / background"),
            ] {
                ui.selectable_value(&mut self.theme_section, section, label);
            }
        });
        ui.separator();

        let infos = self.theme_infos.clone();
        let mut theme = self.theme.clone();
        let mut load_id: Option<i64> = None;
        let mut save = false;
        let display = self.theme_display;
        let section = self.theme_section;

        let style = ui.style().clone();
        ui.columns(2, |columns| {
            columns[0].set_width(240.0);
            egui::Frame::group(&style).show(&mut columns[0], |ui| {
                ui.heading("Themes");
                egui::ScrollArea::vertical()
                    .id_salt("themes-list")
                    .max_height(ui.available_height() - 8.0)
                    .show(ui, |ui| {
                        for info in &infos {
                            if ui
                                .selectable_label(
                                    info.theme_id == theme.theme_id,
                                    format!(
                                        "{}{}",
                                        info.name,
                                        if info.theme_id == theme.theme_id {
                                            "  ✓"
                                        } else {
                                            ""
                                        }
                                    ),
                                )
                                .clicked()
                            {
                                load_id = Some(info.theme_id);
                            }
                        }
                    });
            });

            egui::Frame::group(&style).show(&mut columns[1], |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Theme");
                    ui.label(
                        RichText::new(format!("(#{} • display {})", theme.theme_id, display))
                            .weak(),
                    );
                });
                egui::Grid::new("theme-meta")
                    .num_columns(2)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut theme.name);
                        ui.end_row();
                        ui.label("Comments");
                        ui.text_edit_singleline(&mut theme.comments);
                        ui.end_row();
                    });
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("theme-editor")
                    .auto_shrink([false, false])
                    .show(ui, |ui| match (section, display) {
                        (ThemeSection::Bible, 1) => bible_editor(ui, &mut theme.bible),
                        (ThemeSection::Bible, _) => bible_editor(ui, &mut theme.bible2),
                        (ThemeSection::Song, 1) => song_editor(ui, &mut theme.song),
                        (ThemeSection::Song, _) => song_editor(ui, &mut theme.song2),
                        (ThemeSection::Announce, 1) => announce_editor(ui, &mut theme.announce),
                        (ThemeSection::Announce, _) => announce_editor(ui, &mut theme.announce2),
                        (ThemeSection::Passive, 1) => passive_editor(ui, &mut theme.passive),
                        (ThemeSection::Passive, _) => passive_editor(ui, &mut theme.passive2),
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("💾 Save theme").clicked() {
                        save = true;
                    }
                    if ui.button("↺ Reload").clicked() {
                        load_id = Some(theme.theme_id);
                    }
                    ui.label(
                        RichText::new("Changes are applied immediately to new projections.").weak(),
                    );
                });
            });
        });

        if let Some(id) = load_id {
            self.load_theme(id);
        } else {
            if save {
                match theme.save_update(&self.db) {
                    Ok(()) => {
                        self.settings.general.current_theme_id = theme.theme_id;
                        let _ = self.settings.save(&self.db);
                        self.images.clear();
                        self.reload_themes();
                        self.set_status("Theme saved");
                    }
                    Err(err) => self.set_error("Could not save theme", err),
                }
            }
            self.theme = theme;
        }
    }

    pub fn import_themes_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SoftProjector theme", &["spt"])
            .add_filter("All files", &["*"])
            .pick_file()
        else {
            return;
        };
        match import_themes(&self.db, &path) {
            Ok(count) => {
                self.reload_themes();
                self.set_status(format!("Imported {count} theme(s)"));
            }
            Err(err) => self.set_error("Could not import themes", err),
        }
    }

    pub fn export_themes_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SoftProjector theme", &["spt"])
            .set_file_name("themes.spt")
            .save_file()
        else {
            return;
        };
        match export_themes(&self.db, &path, None) {
            Ok(()) => self.set_status(format!("Exported themes to {}", path.display())),
            Err(err) => self.set_error("Could not export themes", err),
        }
    }
}

fn bible_editor(ui: &mut egui::Ui, s: &mut BibleSettings) {
    ui.heading("Bible text");
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.use_shadow, "Text shadow");
        ui.checkbox(&mut s.use_fading, "Fade transition");
        ui.checkbox(&mut s.use_blur_shadow, "Blur shadow");
    });
    widgets::font_edit(ui, "Text font", &mut s.text_font);
    widgets::color_edit(ui, "Text color", &mut s.text_color);
    widgets::alignment_edit(ui, "Horizontal alignment", &mut s.text_alignment_h, false);
    widgets::alignment_edit(ui, "Vertical alignment", &mut s.text_alignment_v, true);

    ui.separator();
    ui.heading("Caption");
    widgets::font_edit(ui, "Caption font", &mut s.caption_font);
    widgets::color_edit(ui, "Caption color", &mut s.caption_color);
    widgets::alignment_edit(ui, "Caption alignment", &mut s.caption_alignment, false);
    ui.horizontal(|ui| {
        ui.label("Caption position");
        egui::ComboBox::from_id_salt("bible-caption-position")
            .selected_text(if s.caption_position == 0 {
                "Top"
            } else {
                "Bottom"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.caption_position, 0, "Top");
                ui.selectable_value(&mut s.caption_position, 1, "Bottom");
            });
        ui.checkbox(&mut s.use_abbriviations, "Use abbreviation");
    });

    ui.separator();
    ui.heading("Screen use");
    ui.add(
        egui::Slider::new(&mut s.screen_use, 25..=100)
            .text("Screen fill (%)")
            .suffix("%"),
    );
    ui.horizontal(|ui| {
        ui.label("Vertical position");
        egui::ComboBox::from_id_salt("bible-screen-position")
            .selected_text(match s.screen_position {
                0 => "Top",
                2 => "Bottom",
                _ => "Center",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.screen_position, 0, "Top");
                ui.selectable_value(&mut s.screen_position, 1, "Center");
                ui.selectable_value(&mut s.screen_position, 2, "Bottom");
            });
    });

    ui.separator();
    widgets::background_edit(
        ui,
        &mut s.use_background,
        &mut s.background_name,
        &mut s.background,
    );
}

fn song_editor(ui: &mut egui::Ui, s: &mut SongSettings) {
    ui.heading("Song text");
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.use_shadow, "Text shadow");
        ui.checkbox(&mut s.use_fading, "Fade transition");
        ui.checkbox(&mut s.use_blur_shadow, "Blur shadow");
    });
    widgets::font_edit(ui, "Text font", &mut s.text_font);
    widgets::color_edit(ui, "Text color", &mut s.text_color);
    widgets::alignment_edit(ui, "Horizontal alignment", &mut s.text_alignment_h, false);
    widgets::alignment_edit(ui, "Vertical alignment", &mut s.text_alignment_v, true);
    ui.add(egui::Slider::new(&mut s.screen_use, 25..=100).text("Screen fill (%)"));

    ui.separator();
    ui.heading("Song information");
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.show_stanza_title, "Show stanza title");
        ui.checkbox(&mut s.show_song_number, "Show song number");
        ui.checkbox(&mut s.show_song_key, "Show key");
    });
    widgets::font_edit(ui, "Info font", &mut s.info_font);
    widgets::color_edit(ui, "Info color", &mut s.info_color);
    ui.horizontal(|ui| {
        ui.label("Info position");
        egui::ComboBox::from_id_salt("song-info-position")
            .selected_text(if s.info_align == 0 { "Top" } else { "Bottom" })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.info_align, 0, "Top");
                ui.selectable_value(&mut s.info_align, 1, "Bottom");
            });
    });

    ui.separator();
    ui.heading("Song ending");
    ui.checkbox(&mut s.show_song_ending, "Show ending on the last stanza");
    widgets::font_edit(ui, "Ending font", &mut s.ending_font);
    widgets::color_edit(ui, "Ending color", &mut s.ending_color);
    ui.horizontal(|ui| {
        ui.label("Ending");
        egui::ComboBox::from_id_salt("song-ending-type")
            .selected_text(ENDING_NAMES[s.ending_type.clamp(0, 7) as usize])
            .show_ui(ui, |ui| {
                for (index, name) in ENDING_NAMES.iter().enumerate() {
                    ui.selectable_value(&mut s.ending_type, index as i32, *name);
                }
            });
        ui.label("Position");
        egui::ComboBox::from_id_salt("song-ending-position")
            .selected_text(if s.ending_position == 0 {
                "Top"
            } else {
                "Bottom"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.ending_position, 0, "Top");
                ui.selectable_value(&mut s.ending_position, 1, "Bottom");
            });
    });

    ui.separator();
    widgets::background_edit(
        ui,
        &mut s.use_background,
        &mut s.background_name,
        &mut s.background,
    );
}

const ENDING_NAMES: [&str; 8] = [
    "Asterisks",
    "Dashes",
    "Music notes",
    "Hearts",
    "Cross",
    "Orthodox cross",
    "Fleuron",
    "Words / Music by",
];

fn announce_editor(ui: &mut egui::Ui, s: &mut AnnounceSettings) {
    ui.heading("Announcement text");
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.use_shadow, "Text shadow");
        ui.checkbox(&mut s.use_fading, "Fade transition");
        ui.checkbox(&mut s.use_blur_shadow, "Blur shadow");
    });
    widgets::font_edit(ui, "Text font", &mut s.text_font);
    widgets::color_edit(ui, "Text color", &mut s.text_color);
    widgets::alignment_edit(ui, "Horizontal alignment", &mut s.text_alignment_h, false);
    widgets::alignment_edit(ui, "Vertical alignment", &mut s.text_alignment_v, true);
    ui.separator();
    widgets::background_edit(
        ui,
        &mut s.use_background,
        &mut s.background_name,
        &mut s.background,
    );
}

fn passive_editor(ui: &mut egui::Ui, s: &mut PassiveSettings) {
    ui.heading("Passive display");
    ui.label(
        RichText::new(
            "The passive view is shown when nothing is being projected. It usually contains a logo or a welcome background.",
        )
        .weak(),
    );
    ui.separator();
    widgets::background_edit(
        ui,
        &mut s.use_background,
        &mut s.background_name,
        &mut s.background,
    );
}
