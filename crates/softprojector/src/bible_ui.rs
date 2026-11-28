//! Bible browsing, search and projection.

use crate::app::App;
use crate::project::{Projected, TextSlide, TextStyle};
use crate::widgets;
use eframe::egui::{self, RichText};
use sp_core::bible::{build_verse_and_caption, BibleHistory};
use sp_core::import_export::{export_bible, import_bible};
use sp_core::{BibleSession, ScheduleItem};

impl App {
    pub fn bible_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Біблія");
            ui.separator();
            ui.label("Версія:");
            let version_name = self
                .bible_versions
                .iter()
                .find(|v| v.id == self.bible_version_id)
                .map(|v| v.name.clone())
                .unwrap_or_else(|| "Біблію не завантажено".into());
            egui::ComboBox::from_id_salt("bible-version")
                .selected_text(version_name)
                .width(240.0)
                .show_ui(ui, |ui| {
                    for version in &self.bible_versions {
                        if ui
                            .selectable_value(
                                &mut self.bible_version_id,
                                version.id,
                                format!("{} ({})", version.name, version.abbreviation),
                            )
                            .changed()
                        {
                            self.bible_extra_sessions.clear();
                        }
                    }
                });
            if ui.button("📥 Імпортувати Біблію...").clicked() {
                self.import_bible_dialog();
            }
            if ui.button("Керування версіями...").clicked() {
                self.show_bible_manager = true;
            }
            ui.separator();
            if ui.button("▶ Проєктувати вибране").clicked() {
                self.project_bible_selection();
            }
            if ui.button("Додати до розкладу").clicked() {
                self.add_bible_to_schedule();
            }
        });

        if self.bible_versions.is_empty() {
            ui.separator();
            ui.label(
                RichText::new(
                    "Не встановлено жодної версії Біблії. Скористайтеся «Імпортувати Біблію...», щоб завантажити файл .spb.",
                )
                .color(egui::Color32::YELLOW),
            );
            return;
        }

        let Some(session) = self.bible_session.as_ref() else {
            ui.label("Завантаження Біблії...");
            return;
        };

        let books = session.books.clone();
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Книга:");
            let current_book_name = books
                .iter()
                .find(|b| b.book_id == self.bible_book)
                .map(|b| b.book.clone())
                .unwrap_or_else(|| "—".into());
            egui::ComboBox::from_id_salt("bible-book")
                .selected_text(current_book_name)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for book in &books {
                        if ui
                            .selectable_value(
                                &mut self.bible_book,
                                book.book_id,
                                format!("{} (розд.: {})", book.book, book.chapter_count),
                            )
                            .clicked()
                        {
                            self.bible_chapter = 1;
                        }
                    }
                });

            let max_chapter = books
                .iter()
                .find(|b| b.book_id == self.bible_book)
                .map(|b| b.chapter_count.max(1))
                .unwrap_or(1);
            if ui.button("◀").clicked() {
                self.bible_chapter = (self.bible_chapter - 1).max(1);
            }
            ui.add(
                egui::DragValue::new(&mut self.bible_chapter)
                    .range(1..=max_chapter)
                    .prefix("Розділ "),
            );
            if ui.button("▶").clicked() {
                self.bible_chapter = (self.bible_chapter + 1).min(max_chapter);
            }
            ui.separator();
            ui.label(format!("Вибрано віршів: {}", self.bible_selection.len()));
            if ui.button("Вибрати розділ").clicked() {
                if let Some(session) = self.bible_session.as_mut() {
                    let _ = session.chapter(self.bible_book, self.bible_chapter);
                    self.bible_selection = session.preview_id_list.clone();
                }
            }
            if ui.button("Очистити").clicked() {
                self.bible_selection.clear();
            }
        });

        let style = ui.style().clone();
        ui.columns(2, |columns| {
            columns[0].set_width(360.0);
            egui::Frame::group(&style).show(&mut columns[0], |ui| {
                ui.heading("Розділ");
                let (verses, ids) = if let Some(session) = self.bible_session.as_mut() {
                    let verses = session.chapter(self.bible_book, self.bible_chapter);
                    (verses, session.preview_id_list.clone())
                } else {
                    (Vec::new(), Vec::new())
                };
                egui::ScrollArea::vertical()
                    .id_salt("bible-verses")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if verses.is_empty() {
                            ui.label(RichText::new("У цьому розділі немає віршів.").weak());
                        }
                        for (index, verse) in verses.iter().enumerate() {
                            let id = ids.get(index).cloned().unwrap_or_default();
                            let selected = self.bible_selection.contains(&id);
                            let response =
                                ui.selectable_label(selected, format!("{}. {}", index + 1, verse));
                            if response.clicked() {
                                toggle_selection(&mut self.bible_selection, id);
                            }
                        }
                    });
            });

            egui::Frame::group(&style).show(&mut columns[1], |ui| {
                ui.heading("Пошук");
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.bible_search)
                            .desired_width(260.0)
                            .hint_text("слово або фраза"),
                    );
                    let search = ui.button("🔍 Знайти").clicked()
                        || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                    if search {
                        self.run_bible_search();
                    }
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.bible_all_words, "Усі слова");
                    ui.checkbox(&mut self.bible_scope_current, "Лише поточний розділ");
                    if !self.bible_results.is_empty() {
                        ui.label(format!("Результатів: {}", self.bible_results.len()));
                    }
                });
                ui.separator();

                let mut add_id: Option<String> = None;
                egui::ScrollArea::vertical()
                    .id_salt("bible-search-results")
                    .max_height(ui.available_height() - 8.0)
                    .show(ui, |ui| {
                        if self.bible_results.is_empty() {
                            ui.label(
                                RichText::new(
                                    "Тут з'являться результати пошуку. Натисніть на результат, щоб додати вірш до вибраного.",
                                )
                                .weak(),
                            );
                        }
                        for result in &self.bible_results {
                            let label = format!(
                                "{} {}:{}  {}",
                                result.book, result.chapter, result.verse, result.verse_text
                            );
                            if ui.selectable_label(false, label).clicked() {
                                add_id = Some(result.verse_id.clone());
                            }
                        }
                    });
                if let Some(id) = add_id {
                    if let Some(session) = self.bible_session.as_ref() {
                        if let Some((book, chapter, _)) = session.verse_ref(&id) {
                            if let Some(book_row) = session.books.iter().find(|b| b.book == book) {
                                self.bible_book = book_row.book_id;
                            }
                            self.bible_chapter = chapter;
                        }
                    }
                    toggle_selection(&mut self.bible_selection, id);
                }
            });
        });
    }

    fn run_bible_search(&mut self) {
        let query = self.bible_search.trim().to_string();
        if query.is_empty() {
            self.bible_results.clear();
            return;
        }
        let words: Vec<String> = query
            .split_whitespace()
            .map(|word| regex::escape(word))
            .collect();
        if words.is_empty() {
            return;
        }
        let pattern = if self.bible_all_words {
            format!("\\b({})\\b", words.join("|"))
        } else {
            words.join("\\W*")
        };
        let book = self.bible_scope_current.then_some(self.bible_book);
        let chapter = self.bible_scope_current.then_some(self.bible_chapter);
        let Some(session) = self.bible_session.as_ref() else {
            return;
        };
        self.bible_results = session.search(self.bible_all_words, &pattern, book, chapter);
        self.set_status(format!(
            "Пошук у Біблії: результатів — {}",
            self.bible_results.len()
        ));
    }

    pub fn project_bible_selection(&mut self) {
        if self.bible_selection.is_empty() {
            self.set_error(
                "Нічого проєктувати",
                "спершу виберіть один або кілька віршів",
            );
            return;
        }
        let verse_ids = self.bible_selection.join(",");
        self.project_bible_ids(&verse_ids, "");
    }

    /// Project a comma separated list of verse ids, used by the schedule too.
    pub fn project_bible_ids(&mut self, verse_ids: &str, source_hint: &str) {
        let Some(session) = self.bible_session.clone() else {
            self.set_error("Нічого проєктувати", "версію Біблії не завантажено");
            return;
        };
        let verse_ids = verse_ids.to_string();
        let (primary_text, primary_caption) = build_verse_and_caption(
            &session.verses,
            &session.books,
            &session.version.abbreviation,
            &verse_ids,
            self.theme.bible.use_abbriviations,
        );

        let mut text_parts = vec![primary_text.clone()];
        let mut captions = vec![primary_caption.clone()];

        let extra_versions = [
            self.settings.bible_sets.secondary_bible.clone(),
            self.settings.bible_sets.trinary_bible.clone(),
        ];
        for version_id in extra_versions {
            let Ok(id) = version_id.parse::<i64>() else {
                continue;
            };
            if id == session.version.id {
                continue;
            }
            if !self.bible_extra_sessions.contains_key(&id) {
                match BibleSession::load(&self.db, id) {
                    Ok(Some(extra)) => {
                        self.bible_extra_sessions.insert(id, extra);
                    }
                    Ok(None) => continue,
                    Err(err) => {
                        self.set_error("Не вдалося завантажити додаткову версію Біблії", err);
                        continue;
                    }
                }
            }
            if let Some(extra) = self.bible_extra_sessions.get(&id) {
                let (text, caption) = build_verse_and_caption(
                    &extra.verses,
                    &extra.books,
                    &extra.version.abbreviation,
                    &verse_ids,
                    self.theme.bible.use_abbriviations,
                );
                if !text.trim().is_empty() && !text_parts.contains(&text) {
                    text_parts.push(text);
                    if !caption.trim().is_empty() {
                        captions.push(caption);
                    }
                }
            }
        }

        let mut text = text_parts.join("\n\n");
        if text.trim().is_empty() {
            text = format!(
                "Вибраних віршів ({}) не знайдено в цій версії Біблії.",
                verse_ids
            );
        }
        let caption = captions
            .into_iter()
            .filter(|caption| !caption.trim().is_empty())
            .collect::<Vec<_>>()
            .join("  •  ");

        let sets = &self.theme.bible;
        let style = TextStyle {
            font: sets.text_font.clone(),
            color: sets.text_color,
            align_h: sets.text_alignment_h,
            align_v: sets.text_alignment_v,
            screen_use: sets.screen_use,
            screen_position: sets.screen_position,
            shadow: sets.use_shadow,
            caption_font: sets.caption_font.clone(),
            caption_color: sets.caption_color,
            caption_align_h: sets.caption_alignment,
            caption_at_top: sets.caption_position == 0,
            info_font: sets.caption_font.clone(),
            info_color: sets.caption_color,
            info_at_top: true,
            ending_font: sets.caption_font.clone(),
            ending_color: sets.caption_color,
            ending_at_top: false,
            background: if sets.use_background {
                sets.background.clone()
            } else {
                None
            },
        };

        let source = if source_hint.trim().is_empty() {
            format!("Біблія — {caption}")
        } else {
            format!("Біблія — {source_hint}")
        };
        self.start_presentation(
            source,
            vec![Projected::Text(TextSlide {
                text,
                caption: caption.clone(),
                corner_left: String::new(),
                corner_right: String::new(),
                ending: String::new(),
                style,
                label: caption,
            })],
            0,
        );
    }

    pub fn add_bible_to_schedule(&mut self) {
        if self.bible_selection.is_empty() {
            self.set_error(
                "Не вдалося додати до розкладу",
                "спершу виберіть один або кілька віршів",
            );
            return;
        }
        let Some(session) = self.bible_session.as_ref() else {
            return;
        };
        let ids = self.bible_selection.join(",");
        let (_, caption) = build_verse_and_caption(
            &session.verses,
            &session.books,
            &session.version.abbreviation,
            &ids,
            self.theme.bible.use_abbriviations,
        );
        self.schedule.push(ScheduleItem::from_bible(BibleHistory {
            verse_ids: ids,
            caption_long: caption.clone(),
            caption,
        }));
        self.selected_schedule = self.schedule.len().saturating_sub(1);
        self.set_status("Вірші додано до розкладу");
    }

    pub fn bible_manager_window(&mut self, ctx: &egui::Context) {
        if !self.show_bible_manager {
            return;
        }
        let mut open = self.show_bible_manager;
        egui::Window::new("Версії Біблії")
            .open(&mut open)
            .default_width(560.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for version in self.bible_versions.clone() {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&version.name).strong());
                                ui.label(
                                    RichText::new(format!(
                                        "{} • книг: {}",
                                        version.abbreviation,
                                        self.bible_versions.len()
                                    ))
                                    .weak(),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("Видалити").clicked() {
                                            match sp_core::BibleVersion::delete(
                                                &self.db, version.id,
                                            ) {
                                                Ok(()) => {
                                                    self.bible_extra_sessions.remove(&version.id);
                                                    self.reload_bible_versions();
                                                    self.set_status("Версію Біблії видалено");
                                                }
                                                Err(err) => self.set_error(
                                                    "Не вдалося видалити версію Біблії",
                                                    err,
                                                ),
                                            }
                                        }
                                        if ui.small_button("Експортувати...").clicked()
                                        {
                                            self.export_bible_dialog(version.id);
                                        }
                                    },
                                );
                            });
                            if !version.information.trim().is_empty() {
                                ui.label(
                                    RichText::new(widgets::one_line(&version.information)).weak(),
                                );
                            }
                        }
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("📥 Імпортувати .spb...").clicked() {
                        self.import_bible_dialog();
                    }
                    if ui.button("Експортувати вибране...").clicked() {
                        self.export_bible_dialog(self.bible_version_id);
                    }
                });
            });
        self.show_bible_manager = open;
    }

    pub fn import_bible_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Біблія SoftProjector", &["spb"])
            .add_filter("Усі файли", &["*"])
            .pick_file()
        else {
            return;
        };
        match import_bible(&self.db, &path) {
            Ok(id) => {
                self.bible_extra_sessions.clear();
                self.reload_bible_versions();
                self.bible_version_id = id;
                self.bible_session = None;
                self.ensure_bible_session();
                self.set_status(format!("Біблію імпортовано з {}", path.display()));
            }
            Err(err) => self.set_error("Не вдалося імпортувати Біблію", err),
        }
    }

    pub fn export_bible_dialog(&mut self, version_id: i64) {
        if version_id <= 0 {
            self.set_error("Не вдалося експортувати Біблію", "версію Біблії не вибрано");
            return;
        }
        let name = self
            .bible_versions
            .iter()
            .find(|v| v.id == version_id)
            .map(|v| format!("{}.spb", v.name))
            .unwrap_or_else(|| "bible.spb".into());
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Біблія SoftProjector", &["spb"])
            .set_file_name(name)
            .save_file()
        else {
            return;
        };
        match export_bible(&self.db, version_id, &path) {
            Ok(()) => self.set_status(format!("Біблію експортовано до {}", path.display())),
            Err(err) => self.set_error("Не вдалося експортувати Біблію", err),
        }
    }
}

fn toggle_selection(selection: &mut Vec<String>, id: String) {
    if id.trim().is_empty() {
        return;
    }
    if let Some(position) = selection.iter().position(|existing| *existing == id) {
        selection.remove(position);
    } else {
        selection.push(id);
    }
}
