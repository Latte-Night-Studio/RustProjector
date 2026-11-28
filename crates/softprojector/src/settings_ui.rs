//! General application settings.

use crate::app::App;
use crate::widgets;
use eframe::egui::{self, RichText};

impl App {
    pub fn settings_tab(&mut self, ui: &mut egui::Ui) {
        let mut check_update = false;
        let mut download_update = false;
        let mut install_update = false;

        ui.heading("Налаштування");
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("settings-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.heading("Дисплей проєкції");
                egui::Grid::new("settings-general")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Основний дисплей");
                        let current =
                            display_name(&self.monitors, self.settings.general.display_screen);
                        egui::ComboBox::from_id_salt("settings-screen")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                if self.monitors.is_empty() {
                                    ui.label("Монітори не знайдено");
                                }
                                for (index, (x, y, width, height)) in
                                    self.monitors.iter().copied().enumerate()
                                {
                                    ui.selectable_value(
                                        &mut self.settings.general.display_screen,
                                        index as i32,
                                        format!(
                                            "Дисплей {} — {}×{} у ({}, {})",
                                            index + 1,
                                            width,
                                            height,
                                            x,
                                            y
                                        ),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Додатковий дисплей");
                        let mut screen2 = self.settings.general.display_screen2;
                        let second_text = if screen2 < 0 {
                            "Немає".to_string()
                        } else {
                            display_name(&self.monitors, screen2)
                        };
                        egui::ComboBox::from_id_salt("settings-screen2")
                            .selected_text(second_text)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut screen2, -1, "Немає");
                                for (index, _) in self.monitors.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut screen2,
                                        index as i32,
                                        format!("Дисплей {}", index + 1),
                                    );
                                }
                            });
                        self.settings.general.display_screen2 = screen2;
                        ui.end_row();

                        ui.label("Проєктор поверх інших вікон");
                        ui.checkbox(
                            &mut self.settings.general.display_is_on_top,
                            "Завжди поверх",
                        );
                        ui.end_row();

                        ui.label("Тема");
                        let theme_name = self
                            .theme_infos
                            .iter()
                            .find(|t| t.theme_id == self.settings.general.current_theme_id)
                            .map(|t| t.name.clone())
                            .unwrap_or_else(|| self.theme.name.clone());
                        let mut selected_theme = self.settings.general.current_theme_id;
                        egui::ComboBox::from_id_salt("settings-theme")
                            .selected_text(theme_name)
                            .show_ui(ui, |ui| {
                                for info in &self.theme_infos {
                                    ui.selectable_value(
                                        &mut selected_theme,
                                        info.theme_id,
                                        &info.name,
                                    );
                                }
                            });
                        if selected_theme != self.settings.general.current_theme_id
                            && selected_theme > 0
                        {
                            self.load_theme(selected_theme);
                        }
                        ui.end_row();

                        ui.label("Елементи керування");
                        let mut opacity = self.settings.general.display_controls.opacity;
                        ui.horizontal(|ui| {
                            ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0).text("Прозорість"));
                        });
                        self.settings.general.display_controls.opacity = opacity;
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Версії Біблії");
                egui::Grid::new("settings-bibles")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Основна версія");
                        bible_version_combo(
                            ui,
                            "bible-primary",
                            &self.bible_versions,
                            &mut self.settings.bible_sets.primary_bible,
                        );
                        ui.end_row();

                        ui.label("Додаткова версія");
                        bible_version_combo(
                            ui,
                            "bible-secondary",
                            &self.bible_versions,
                            &mut self.settings.bible_sets.secondary_bible,
                        );
                        ui.end_row();

                        ui.label("Третя версія");
                        bible_version_combo(
                            ui,
                            "bible-trinary",
                            &self.bible_versions,
                            &mut self.settings.bible_sets.trinary_bible,
                        );
                        ui.end_row();

                        ui.label("Біблія оператора");
                        bible_version_combo(
                            ui,
                            "bible-operator",
                            &self.bible_versions,
                            &mut self.settings.bible_sets.operator_bible,
                        );
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Слайд-шоу та зображення");
                egui::Grid::new("settings-pix")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Збільшувати малі зображення");
                        ui.checkbox(&mut self.settings.slide_sets.expand_small, "");
                        ui.end_row();

                        ui.label("Режим вписування");
                        egui::ComboBox::from_id_salt("settings-fit-type")
                            .selected_text(match self.settings.slide_sets.fit_type {
                                1 => "Вмістити",
                                2 => "Розтягнути",
                                _ => "Заповнити",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.fit_type,
                                    0,
                                    "Заповнити",
                                );
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.fit_type,
                                    1,
                                    "Вмістити",
                                );
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.fit_type,
                                    2,
                                    "Розтягнути",
                                );
                            });
                        ui.end_row();

                        ui.label("Зменшувати великі зображення");
                        ui.checkbox(&mut self.settings.slide_sets.resize, "");
                        ui.end_row();

                        ui.label("Обмеження розміру");
                        egui::ComboBox::from_id_salt("settings-bound-type")
                            .selected_text(match self.settings.slide_sets.bound_type {
                                0 => "Ширина",
                                1 => "Висота",
                                _ => "Більша сторона",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.bound_type,
                                    0,
                                    "Ширина",
                                );
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.bound_type,
                                    1,
                                    "Висота",
                                );
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.bound_type,
                                    2,
                                    "Більша сторона",
                                );
                            });
                        ui.end_row();

                        ui.label("Максимальний розмір");
                        ui.add(
                            egui::DragValue::new(&mut self.settings.slide_sets.bound_width)
                                .range(320..=7680)
                                .suffix(" px"),
                        );
                        ui.end_row();
                    });

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui.button("💾 Зберегти налаштування").clicked() {
                        match self.settings.save(&self.db) {
                            Ok(()) => self.set_status("Налаштування збережено"),
                            Err(err) => self.set_error("Не вдалося зберегти налаштування", err),
                        }
                    }
                    if ui
                        .button("↺ Перезавантажити збережені налаштування")
                        .clicked()
                    {
                        match sp_core::Settings::load(&self.db) {
                            Ok(settings) => {
                                self.settings = settings;
                                self.set_status("Налаштування перезавантажено");
                            }
                            Err(err) => {
                                self.set_error("Не вдалося перезавантажити налаштування", err)
                            }
                        }
                    }
                });

                ui.add_space(16.0);
                ui.separator();
                ui.heading("База даних");
                ui.label(RichText::new(self.db.path().display().to_string()).monospace());
                ui.label(
                    RichText::new(format!(
                        "Версія схеми {} — сумісна з SoftProjector 2 spData.sqlite",
                        sp_core::DB_VERSION
                    ))
                    .weak(),
                );
                ui.horizontal(|ui| {
                    if ui.button("Відкрити теку даних").clicked() {
                        if let Some(parent) = self.db.path().parent() {
                            widgets::open_external(&parent.to_string_lossy());
                        }
                    }
                });

                ui.add_space(16.0);
                ui.separator();
                ui.heading("Оновлення");
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Поточна версія: v{}",
                        crate::updater::CURRENT_VERSION
                    ));
                    if ui
                        .add_enabled(
                            !self.update.checking && !self.update.downloading,
                            egui::Button::new("Перевірити оновлення"),
                        )
                        .clicked()
                    {
                        check_update = true;
                    }
                    if self.update.checking {
                        ui.spinner();
                        ui.label("Перевірка...");
                    }
                });
                if let Some(info) = self.update.info.clone() {
                    ui.label(format!("Доступна версія: v{}", info.version));
                    if !info.notes.trim().is_empty() {
                        ui.collapsing("Що нового", |ui| {
                            ui.label(RichText::new(info.notes.clone()).weak());
                        });
                    }
                    ui.horizontal(|ui| {
                        if self.update.downloading {
                            ui.spinner();
                            ui.label("Завантаження...");
                        } else if self.update.ready.is_some() {
                            if ui
                                .add_enabled(
                                    self.update.can_install(),
                                    egui::Button::new("Перезапустити та встановити"),
                                )
                                .clicked()
                            {
                                install_update = true;
                            }
                        } else if ui.button("Завантажити оновлення").clicked() {
                            download_update = true;
                        }
                        if ui.button("Відкрити реліз").clicked() {
                            widgets::open_external(&info.page_url);
                        }
                    });
                } else if self.update.checked
                    && !self.update.checking
                    && self.update.error.is_none()
                {
                    ui.label("У вас найновіша версія.");
                }
                if let Some(error) = self.update.error.clone() {
                    ui.label(
                        RichText::new(format!("Помилка оновлення: {error}"))
                            .color(egui::Color32::LIGHT_RED),
                    );
                }
                ui.label(
                    RichText::new(
                        "Автоматична перевірка оновлень виконується під час кожного запуску.",
                    )
                    .weak(),
                );
            });

        if check_update {
            self.update.check_now();
        }
        if download_update {
            self.update.download();
        }
        if install_update {
            if let Some(path) = self.update.ready.clone() {
                match crate::updater::launch_installer(&path) {
                    Ok(()) => {
                        self.projector_open = false;
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    Err(error) => self.set_error("Не вдалося встановити оновлення", error),
                }
            }
        }
    }
}

fn display_name(monitors: &[(i32, i32, i32, i32)], index: i32) -> String {
    if index < 0 {
        return "Немає".into();
    }
    monitors
        .get(index as usize)
        .map(|(x, y, w, h)| format!("Дисплей {} — {}×{} у ({}, {})", index + 1, w, h, x, y))
        .unwrap_or_else(|| format!("Дисплей {} (не знайдено)", index + 1))
}

fn bible_version_combo(
    ui: &mut egui::Ui,
    id: &str,
    versions: &[sp_core::BibleVersion],
    value: &mut String,
) {
    let current_id = value.parse::<i64>().ok();
    let label = versions
        .iter()
        .find(|v| Some(v.id) == current_id)
        .map(|v| format!("{} ({})", v.name, v.abbreviation))
        .unwrap_or_else(|| "Немає".into());
    egui::ComboBox::from_id_salt(id)
        .selected_text(label)
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(value, "немає".to_string(), "Немає")
                .clicked()
            {
                return;
            }
            for version in versions {
                ui.selectable_value(
                    value,
                    version.id.to_string(),
                    format!("{} ({})", version.name, version.abbreviation),
                );
            }
        });
}
