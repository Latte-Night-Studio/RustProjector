//! Theme list and per-display theme editor.

use crate::app::{App, ThemeSection};
use crate::widgets;
use eframe::egui::{self, RichText};
use sp_core::import_export::{export_themes, import_themes};
use sp_core::theme::{AnnounceSettings, BibleSettings, PassiveSettings, SongSettings, Theme};

impl App {
    pub fn themes_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Теми");
            if ui.button("➕ Нове").clicked() {
                let mut theme = Theme::default();
                theme.name = "Нова тема".into();
                theme.comments.clear();
                match theme.save_new(&self.db) {
                    Ok(()) => {
                        self.reload_themes();
                        self.load_theme(theme.theme_id);
                        self.set_status("Тему створено");
                    }
                    Err(err) => self.set_error("Не вдалося створити тему", err),
                }
            }
            if ui.button("⧉ Дублювати").clicked() {
                let name = format!("{} — копія", self.theme.name);
                match self.theme.duplicate(&self.db, &name, &self.theme.comments) {
                    Ok(id) => {
                        self.reload_themes();
                        self.load_theme(id);
                        self.set_status("Тему продубльовано");
                    }
                    Err(err) => self.set_error("Не вдалося продублювати тему", err),
                }
            }
            if ui.button("🗑 Видалити").clicked() {
                if self.theme_infos.len() <= 1 {
                    self.set_error("Не вдалося видалити тему", "потрібна принаймні одна тема");
                } else {
                    let id = self.theme.theme_id;
                    match Theme::delete(&self.db, id) {
                        Ok(()) => {
                            self.reload_themes();
                            let next = self.theme_infos.first().map(|t| t.theme_id).unwrap_or(0);
                            self.load_theme(next);
                            self.set_status("Тему видалено");
                        }
                        Err(err) => self.set_error("Не вдалося видалити тему", err),
                    }
                }
            }
            ui.separator();
            if ui.button("📥 Імпортувати теми...").clicked() {
                self.import_themes_dialog();
            }
            if ui.button("📤 Експортувати всі теми...").clicked() {
                self.export_themes_dialog();
            }
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Редагування дисплея:");
            ui.selectable_value(&mut self.theme_display, 1, "Дисплей 1");
            ui.selectable_value(&mut self.theme_display, 2, "Дисплей 2");
            ui.separator();
            for (section, label) in [
                (ThemeSection::Bible, "Біблія"),
                (ThemeSection::Song, "Пісні"),
                (ThemeSection::Announce, "Оголошення"),
                (ThemeSection::Passive, "Пасивний / фон"),
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
                ui.heading("Теми");
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
                    ui.heading("Тема");
                    ui.label(
                        RichText::new(format!("(#{} • дисплей {})", theme.theme_id, display))
                            .weak(),
                    );
                });
                egui::Grid::new("theme-meta")
                    .num_columns(2)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Назва");
                        ui.text_edit_singleline(&mut theme.name);
                        ui.end_row();
                        ui.label("Коментарі");
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
                    if ui.button("💾 Зберегти тему").clicked() {
                        save = true;
                    }
                    if ui.button("↺ Перезавантажити").clicked() {
                        load_id = Some(theme.theme_id);
                    }
                    ui.label(
                        RichText::new("Зміни одразу застосовуються до нових проєкцій.").weak(),
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
                        self.set_status("Тему збережено");
                    }
                    Err(err) => self.set_error("Не вдалося зберегти тему", err),
                }
            }
            self.theme = theme;
        }
    }

    pub fn import_themes_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Тема SoftProjector", &["spt"])
            .add_filter("Усі файли", &["*"])
            .pick_file()
        else {
            return;
        };
        match import_themes(&self.db, &path) {
            Ok(count) => {
                self.reload_themes();
                self.set_status(format!("Імпортовано тем: {count}"));
            }
            Err(err) => self.set_error("Не вдалося імпортувати теми", err),
        }
    }

    pub fn export_themes_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Тема SoftProjector", &["spt"])
            .set_file_name("themes.spt")
            .save_file()
        else {
            return;
        };
        match export_themes(&self.db, &path, None) {
            Ok(()) => self.set_status(format!("Теми експортовано до {}", path.display())),
            Err(err) => self.set_error("Не вдалося експортувати теми", err),
        }
    }
}

fn bible_editor(ui: &mut egui::Ui, s: &mut BibleSettings) {
    ui.heading("Текст Біблії");
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.use_shadow, "Тінь тексту");
        ui.checkbox(&mut s.use_fading, "Плавний перехід");
        ui.checkbox(&mut s.use_blur_shadow, "Розмита тінь");
    });
    widgets::font_edit(ui, "Шрифт тексту", &mut s.text_font);
    widgets::color_edit(ui, "Колір тексту", &mut s.text_color);
    widgets::alignment_edit(
        ui,
        "Горизонтальне вирівнювання",
        &mut s.text_alignment_h,
        false,
    );
    widgets::alignment_edit(
        ui,
        "Вертикальне вирівнювання",
        &mut s.text_alignment_v,
        true,
    );

    ui.separator();
    ui.heading("Підпис");
    widgets::font_edit(ui, "Шрифт підпису", &mut s.caption_font);
    widgets::color_edit(ui, "Колір підпису", &mut s.caption_color);
    widgets::alignment_edit(ui, "Вирівнювання підпису", &mut s.caption_alignment, false);
    ui.horizontal(|ui| {
        ui.label("Позиція підпису");
        egui::ComboBox::from_id_salt("bible-caption-position")
            .selected_text(if s.caption_position == 0 {
                "Вгорі"
            } else {
                "Внизу"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.caption_position, 0, "Вгорі");
                ui.selectable_value(&mut s.caption_position, 1, "Внизу");
            });
        ui.checkbox(&mut s.use_abbriviations, "Використовувати скорочення");
    });

    ui.separator();
    ui.heading("Використання екрана");
    ui.add(
        egui::Slider::new(&mut s.screen_use, 25..=100)
            .text("Заповнення екрана (%)")
            .suffix("%"),
    );
    ui.horizontal(|ui| {
        ui.label("Вертикальна позиція");
        egui::ComboBox::from_id_salt("bible-screen-position")
            .selected_text(match s.screen_position {
                0 => "Вгорі",
                2 => "Внизу",
                _ => "По центру",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.screen_position, 0, "Вгорі");
                ui.selectable_value(&mut s.screen_position, 1, "По центру");
                ui.selectable_value(&mut s.screen_position, 2, "Внизу");
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
    ui.heading("Текст пісні");
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.use_shadow, "Тінь тексту");
        ui.checkbox(&mut s.use_fading, "Плавний перехід");
        ui.checkbox(&mut s.use_blur_shadow, "Розмита тінь");
    });
    widgets::font_edit(ui, "Шрифт тексту", &mut s.text_font);
    widgets::color_edit(ui, "Колір тексту", &mut s.text_color);
    widgets::alignment_edit(
        ui,
        "Горизонтальне вирівнювання",
        &mut s.text_alignment_h,
        false,
    );
    widgets::alignment_edit(
        ui,
        "Вертикальне вирівнювання",
        &mut s.text_alignment_v,
        true,
    );
    ui.add(egui::Slider::new(&mut s.screen_use, 25..=100).text("Заповнення екрана (%)"));

    ui.separator();
    ui.heading("Інформація про пісню");
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.show_stanza_title, "Показувати назву куплету");
        ui.checkbox(&mut s.show_song_number, "Показувати номер пісні");
        ui.checkbox(&mut s.show_song_key, "Показувати тональність");
    });
    widgets::font_edit(ui, "Шрифт інформації", &mut s.info_font);
    widgets::color_edit(ui, "Колір інформації", &mut s.info_color);
    ui.horizontal(|ui| {
        ui.label("Позиція інформації");
        egui::ComboBox::from_id_salt("song-info-position")
            .selected_text(if s.info_align == 0 {
                "Вгорі"
            } else {
                "Внизу"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.info_align, 0, "Вгорі");
                ui.selectable_value(&mut s.info_align, 1, "Внизу");
            });
    });

    ui.separator();
    ui.heading("Закінчення пісні");
    ui.checkbox(
        &mut s.show_song_ending,
        "Показувати закінчення на останньому куплеті",
    );
    widgets::font_edit(ui, "Шрифт закінчення", &mut s.ending_font);
    widgets::color_edit(ui, "Колір закінчення", &mut s.ending_color);
    ui.horizontal(|ui| {
        ui.label("Закінчення");
        egui::ComboBox::from_id_salt("song-ending-type")
            .selected_text(ENDING_NAMES[s.ending_type.clamp(0, 7) as usize])
            .show_ui(ui, |ui| {
                for (index, name) in ENDING_NAMES.iter().enumerate() {
                    ui.selectable_value(&mut s.ending_type, index as i32, *name);
                }
            });
        ui.label("Позиція");
        egui::ComboBox::from_id_salt("song-ending-position")
            .selected_text(if s.ending_position == 0 {
                "Вгорі"
            } else {
                "Внизу"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.ending_position, 0, "Вгорі");
                ui.selectable_value(&mut s.ending_position, 1, "Внизу");
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
    "Зірочки",
    "Рисочки",
    "Ноти",
    "Сердечка",
    "Хрест",
    "Православний хрест",
    "Орнамент",
    "Слова / музика",
];

fn announce_editor(ui: &mut egui::Ui, s: &mut AnnounceSettings) {
    ui.heading("Текст оголошення");
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.use_shadow, "Тінь тексту");
        ui.checkbox(&mut s.use_fading, "Плавний перехід");
        ui.checkbox(&mut s.use_blur_shadow, "Розмита тінь");
    });
    widgets::font_edit(ui, "Шрифт тексту", &mut s.text_font);
    widgets::color_edit(ui, "Колір тексту", &mut s.text_color);
    widgets::alignment_edit(
        ui,
        "Горизонтальне вирівнювання",
        &mut s.text_alignment_h,
        false,
    );
    widgets::alignment_edit(
        ui,
        "Вертикальне вирівнювання",
        &mut s.text_alignment_v,
        true,
    );
    ui.separator();
    widgets::background_edit(
        ui,
        &mut s.use_background,
        &mut s.background_name,
        &mut s.background,
    );
}

fn passive_editor(ui: &mut egui::Ui, s: &mut PassiveSettings) {
    ui.heading("Пасивний показ");
    ui.label(
        RichText::new(
            "Пасивний показ відображається, коли нічого не проєктується. Зазвичай це логотип або вітальний фон.",
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
