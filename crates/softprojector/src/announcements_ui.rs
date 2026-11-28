//! Announcements.

use crate::app::App;
use crate::project::{Projected, TextSlide, TextStyle};
use crate::widgets;
use eframe::egui::{self, RichText};
use sp_core::{AnnounceSlide, Announcement, ScheduleItem};

impl App {
    pub fn announcements_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Оголошення");
            if ui.button("➕ Нове").clicked() {
                self.announcement_editor = Some(Announcement::new(0));
            }
            if ui.button("💾 Зберегти").clicked() {
                self.save_announcement_editor();
            }
            if ui.button("🗑 Видалити").clicked() {
                self.delete_announcement();
            }
            ui.separator();
            if ui.button("▶ Проєктувати").clicked() {
                self.project_announcement_at(0);
            }
            if ui.button("Додати до розкладу").clicked() {
                if let Some(announcement) = self.announcement_editor.clone() {
                    self.schedule
                        .push(ScheduleItem::from_announcement(announcement));
                    self.selected_schedule = self.schedule.len().saturating_sub(1);
                    self.set_status("Оголошення додано до розкладу");
                }
            }
        });
        ui.separator();

        let style = ui.style().clone();
        ui.columns(2, |columns| {
            columns[0].set_width(280.0);
            egui::Frame::group(&style).show(&mut columns[0], |ui| {
                ui.heading("Усі оголошення");
                egui::ScrollArea::vertical()
                    .id_salt("announcements-list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.announcements.is_empty() {
                            ui.label(RichText::new("Оголошень ще немає.").weak());
                        }
                        for announcement in &self.announcements {
                            let selected = self
                                .announcement_editor
                                .as_ref()
                                .is_some_and(|a| a.id == announcement.id);
                            if ui
                                .selectable_label(
                                    selected,
                                    format!(
                                        "{}{}",
                                        if announcement.title.trim().is_empty() {
                                            "(без назви)"
                                        } else {
                                            announcement.title.trim()
                                        },
                                        if announcement.use_private_settings {
                                            "  •"
                                        } else {
                                            ""
                                        }
                                    ),
                                )
                                .clicked()
                            {
                                self.announcement_editor = Some(announcement.clone());
                            }
                        }
                    });
            });

            egui::Frame::group(&style).show(&mut columns[1], |ui| {
                let Some(editor) = self.announcement_editor.as_mut() else {
                    ui.label("Виберіть оголошення або створіть нове.");
                    return;
                };
                ui.horizontal(|ui| {
                    ui.label("Назва:");
                    ui.add(
                        egui::TextEdit::singleline(&mut editor.title)
                            .desired_width(320.0)
                            .hint_text("Показується у списку оголошень"),
                    );
                });
                ui.add_space(4.0);
                ui.label(
                    "Текст (новий слайд починайте з «Оголошення», «Слайд», «Announce», «Slide»...):",
                );
                let text_edit = egui::TextEdit::multiline(&mut editor.text)
                    .desired_rows(16)
                    .desired_width(f32::INFINITY)
                    .code_editor();
                ui.add(text_edit);

                ui.horizontal(|ui| {
                    ui.checkbox(&mut editor.use_auto_next, "Автоматично наступний слайд");
                    ui.checkbox(&mut editor.loop_slides, "Повторювати слайди");
                    ui.add(
                        egui::DragValue::new(&mut editor.slide_timer)
                            .range(1..=600)
                            .suffix(" s"),
                    );
                });

                ui.separator();
                ui.checkbox(
                    &mut editor.use_private_settings,
                    "Використовувати особисті налаштування для цього оголошення",
                );
                if editor.use_private_settings {
                    widgets::font_edit(ui, "Шрифт", &mut editor.font);
                    widgets::color_edit(ui, "Колір тексту", &mut editor.color);
                    widgets::alignment_edit(ui, "Горизонтально", &mut editor.alignment_h, false);
                    widgets::alignment_edit(ui, "Вертикально", &mut editor.alignment_v, true);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut editor.use_background, "Фонове зображення");
                        if ui.button("Вибрати...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(
                                    "Зображення",
                                    &["jpg", "jpeg", "png", "bmp", "gif", "webp", "tif", "tiff"],
                                )
                                .pick_file()
                            {
                                editor.background_path = path.to_string_lossy().to_string();
                            }
                        }
                        ui.label(if editor.background_path.is_empty() {
                            "немає".into()
                        } else {
                            editor.background_path.clone()
                        });
                    });
                }

                ui.separator();
                ui.label(RichText::new("Слайди").strong());
                let mut project_at: Option<usize> = None;
                egui::ScrollArea::vertical()
                    .id_salt("announcement-slides")
                    .max_height(220.0)
                    .show(ui, |ui| {
                        let list = editor.announce_list();
                        if list.is_empty() {
                            ui.label(RichText::new("Текст оголошення порожній.").weak());
                        }
                        for (index, slide) in list.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if ui.small_button(format!("▶ {}", index + 1)).clicked() {
                                    project_at = Some(index);
                                }
                                ui.label(
                                    RichText::new(widgets::one_line(slide))
                                        .font(egui::FontId::proportional(14.0)),
                                );
                            });
                        }
                    });
                if let Some(index) = project_at {
                    let announcement = editor.clone();
                    self.project_announcement(&announcement, index);
                }
            });
        });
    }

    pub fn save_announcement_editor(&mut self) {
        let Some(mut announcement) = self.announcement_editor.take() else {
            return;
        };
        match announcement.save(&self.db) {
            Ok(()) => {
                self.reload_announcements();
                self.announcement_editor = Some(announcement);
                self.set_status("Оголошення збережено");
            }
            Err(err) => {
                self.set_error("Не вдалося зберегти оголошення", err);
                self.announcement_editor = Some(announcement);
            }
        }
    }

    pub fn delete_announcement(&mut self) {
        let Some(announcement) = self.announcement_editor.as_ref() else {
            return;
        };
        if announcement.id <= 0 {
            self.announcement_editor = None;
            return;
        }
        match Announcement::delete(&self.db, announcement.id) {
            Ok(()) => {
                self.reload_announcements();
                self.announcement_editor = self.announcements.first().cloned();
                self.set_status("Оголошення видалено");
            }
            Err(err) => self.set_error("Не вдалося видалити оголошення", err),
        }
    }

    pub fn project_announcement_at(&mut self, index: usize) {
        if let Some(announcement) = self.announcement_editor.clone() {
            self.project_announcement(&announcement, index);
        } else {
            self.set_error("Нічого проєктувати", "оголошення не вибрано");
        }
    }

    pub fn project_announcement(&mut self, announcement: &Announcement, index: usize) {
        let list = announcement.announce_list();
        if list.is_empty() {
            self.set_error("Нічого проєктувати", "текст оголошення порожній");
            return;
        }

        let sets = &self.theme.announce;
        let mut style = TextStyle {
            font: sets.text_font.clone(),
            color: sets.text_color,
            align_h: sets.text_alignment_h,
            align_v: sets.text_alignment_v,
            screen_use: 100,
            screen_position: 1,
            shadow: sets.use_shadow,
            caption_font: sets.text_font.clone(),
            caption_color: sets.text_color,
            caption_align_h: sets.text_alignment_h,
            caption_at_top: true,
            info_font: sets.text_font.clone(),
            info_color: sets.text_color,
            info_at_top: true,
            ending_font: sets.text_font.clone(),
            ending_color: sets.text_color,
            ending_at_top: false,
            background: if sets.use_background {
                sets.background.clone()
            } else {
                None
            },
        };

        if announcement.use_private_settings {
            style.font = announcement.font.clone();
            style.color = announcement.color;
            style.align_h = announcement.alignment_h;
            style.align_v = announcement.alignment_v;
            if announcement.use_background {
                if let Ok(bytes) = std::fs::read(&announcement.background_path) {
                    style.background = Some(bytes);
                }
            }
        }

        let slides: Vec<Projected> = (0..list.len())
            .map(|index| {
                let slide: AnnounceSlide = announcement.announce_slide(index);
                Projected::Text(TextSlide {
                    text: slide.text,
                    caption: String::new(),
                    corner_left: String::new(),
                    corner_right: String::new(),
                    ending: String::new(),
                    style: style.clone(),
                    label: format!(
                        "{} — слайд {} з {}",
                        if announcement.title.is_empty() {
                            "Оголошення"
                        } else {
                            &announcement.title
                        },
                        index + 1,
                        list.len()
                    ),
                })
            })
            .collect();

        let source = if announcement.title.trim().is_empty() {
            "Оголошення".to_string()
        } else {
            format!("Оголошення — {}", announcement.title)
        };
        self.start_presentation(source, slides, index);
        self.presentation.auto_next = announcement.use_auto_next;
        self.presentation.loop_slides = announcement.loop_slides;
        self.presentation.slide_seconds = announcement.slide_timer.max(1) as f32;
        self.presentation.slide_shown_at = -1.0;
    }
}
