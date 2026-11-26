//! General application settings.

use crate::app::App;
use crate::widgets;
use eframe::egui::{self, RichText};

impl App {
    pub fn settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("settings-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.heading("Projection display");
                egui::Grid::new("settings-general")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Primary display");
                        let current =
                            display_name(&self.monitors, self.settings.general.display_screen);
                        egui::ComboBox::from_id_salt("settings-screen")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                if self.monitors.is_empty() {
                                    ui.label("No monitors detected");
                                }
                                for (index, (x, y, width, height)) in
                                    self.monitors.iter().copied().enumerate()
                                {
                                    ui.selectable_value(
                                        &mut self.settings.general.display_screen,
                                        index as i32,
                                        format!(
                                            "Display {} — {}×{} at ({}, {})",
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

                        ui.label("Secondary display");
                        let mut screen2 = self.settings.general.display_screen2;
                        let second_text = if screen2 < 0 {
                            "None".to_string()
                        } else {
                            display_name(&self.monitors, screen2)
                        };
                        egui::ComboBox::from_id_salt("settings-screen2")
                            .selected_text(second_text)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut screen2, -1, "None");
                                for (index, _) in self.monitors.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut screen2,
                                        index as i32,
                                        format!("Display {}", index + 1),
                                    );
                                }
                            });
                        self.settings.general.display_screen2 = screen2;
                        ui.end_row();

                        ui.label("Keep projector on top");
                        ui.checkbox(
                            &mut self.settings.general.display_is_on_top,
                            "Always on top",
                        );
                        ui.end_row();

                        ui.label("Theme");
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

                        ui.label("Display controls");
                        let mut opacity = self.settings.general.display_controls.opacity;
                        ui.horizontal(|ui| {
                            ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0).text("Opacity"));
                        });
                        self.settings.general.display_controls.opacity = opacity;
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Bible versions");
                egui::Grid::new("settings-bibles")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Primary version");
                        bible_version_combo(
                            ui,
                            "bible-primary",
                            &self.bible_versions,
                            &mut self.settings.bible_sets.primary_bible,
                        );
                        ui.end_row();

                        ui.label("Secondary version");
                        bible_version_combo(
                            ui,
                            "bible-secondary",
                            &self.bible_versions,
                            &mut self.settings.bible_sets.secondary_bible,
                        );
                        ui.end_row();

                        ui.label("Trinary version");
                        bible_version_combo(
                            ui,
                            "bible-trinary",
                            &self.bible_versions,
                            &mut self.settings.bible_sets.trinary_bible,
                        );
                        ui.end_row();

                        ui.label("Operator Bible");
                        bible_version_combo(
                            ui,
                            "bible-operator",
                            &self.bible_versions,
                            &mut self.settings.bible_sets.operator_bible,
                        );
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Slideshows and pictures");
                egui::Grid::new("settings-pix")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Expand small pictures");
                        ui.checkbox(&mut self.settings.slide_sets.expand_small, "");
                        ui.end_row();

                        ui.label("Fit type");
                        egui::ComboBox::from_id_salt("settings-fit-type")
                            .selected_text(match self.settings.slide_sets.fit_type {
                                1 => "Contain",
                                2 => "Stretch",
                                _ => "Cover",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.fit_type,
                                    0,
                                    "Cover",
                                );
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.fit_type,
                                    1,
                                    "Contain",
                                );
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.fit_type,
                                    2,
                                    "Stretch",
                                );
                            });
                        ui.end_row();

                        ui.label("Resize large pictures");
                        ui.checkbox(&mut self.settings.slide_sets.resize, "");
                        ui.end_row();

                        ui.label("Bound type");
                        egui::ComboBox::from_id_salt("settings-bound-type")
                            .selected_text(match self.settings.slide_sets.bound_type {
                                0 => "Width",
                                1 => "Height",
                                _ => "Longest side",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.bound_type,
                                    0,
                                    "Width",
                                );
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.bound_type,
                                    1,
                                    "Height",
                                );
                                ui.selectable_value(
                                    &mut self.settings.slide_sets.bound_type,
                                    2,
                                    "Longest side",
                                );
                            });
                        ui.end_row();

                        ui.label("Maximum size");
                        ui.add(
                            egui::DragValue::new(&mut self.settings.slide_sets.bound_width)
                                .range(320..=7680)
                                .suffix(" px"),
                        );
                        ui.end_row();
                    });

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui.button("💾 Save settings").clicked() {
                        match self.settings.save(&self.db) {
                            Ok(()) => self.set_status("Settings saved"),
                            Err(err) => self.set_error("Could not save settings", err),
                        }
                    }
                    if ui.button("↺ Reload saved settings").clicked() {
                        match sp_core::Settings::load(&self.db) {
                            Ok(settings) => {
                                self.settings = settings;
                                self.set_status("Settings reloaded");
                            }
                            Err(err) => self.set_error("Could not reload settings", err),
                        }
                    }
                });

                ui.add_space(16.0);
                ui.separator();
                ui.heading("Database");
                ui.label(RichText::new(self.db.path().display().to_string()).monospace());
                ui.label(
                    RichText::new(format!(
                        "Schema version {} — compatible with SoftProjector 2 spData.sqlite",
                        sp_core::DB_VERSION
                    ))
                    .weak(),
                );
                ui.horizontal(|ui| {
                    if ui.button("Open data folder").clicked() {
                        if let Some(parent) = self.db.path().parent() {
                            widgets::open_external(&parent.to_string_lossy());
                        }
                    }
                });
            });
    }
}

fn display_name(monitors: &[(i32, i32, i32, i32)], index: i32) -> String {
    if index < 0 {
        return "None".into();
    }
    monitors
        .get(index as usize)
        .map(|(x, y, w, h)| format!("Display {} — {}×{} at ({}, {})", index + 1, w, h, x, y))
        .unwrap_or_else(|| format!("Display {} (not detected)", index + 1))
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
        .unwrap_or_else(|| "None".into());
    egui::ComboBox::from_id_salt(id)
        .selected_text(label)
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(value, "none".to_string(), "None")
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
