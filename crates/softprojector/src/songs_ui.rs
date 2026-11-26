//! Songs, songbooks and the song editor.

use crate::app::{App, Tab};
use crate::project::{song_ending, Projected, TextStyle};
use crate::widgets;
use eframe::egui::{self, RichText};
use sp_core::import_export::{export_songbook, import_songbook};
use sp_core::song::CATEGORIES;
use sp_core::theme::Theme;
use sp_core::{FontSpec, ScheduleItem, Song, Songbook};

pub struct SongEditor {
    pub song: Song,
    pub is_new: bool,
}

impl App {
    pub fn songs_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Songs");
            if ui.button("➕ New song").clicked() {
                self.new_song();
            }
            if ui.button("📚 Songbooks...").clicked() {
                self.show_songbooks = true;
            }
            if ui.button("Import songbook...").clicked() {
                self.import_songbook_dialog();
            }
            ui.separator();
            if let Some(song) = self.selected_song().cloned() {
                if ui.button("▶ Project").clicked() {
                    self.project_song(&song, 0);
                }
                if ui.button("Add to schedule").clicked() {
                    self.add_song_to_schedule(&song);
                }
            }
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.add(
                egui::TextEdit::singleline(&mut self.song_search)
                    .desired_width(220.0)
                    .hint_text("title, number or text"),
            );
            if ui.button("✖").clicked() {
                self.song_search.clear();
            }
            ui.separator();
            ui.label("Songbook:");
            let selected_name = if self.songbook_filter == 0 {
                "All songbooks".to_string()
            } else {
                self.songbooks
                    .iter()
                    .find(|b| b.id == self.songbook_filter)
                    .map(|b| b.name.clone())
                    .unwrap_or_else(|| "All songbooks".into())
            };
            egui::ComboBox::from_id_salt("songbook-filter")
                .selected_text(selected_name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.songbook_filter, 0, "All songbooks");
                    for book in &self.songbooks {
                        ui.selectable_value(&mut self.songbook_filter, book.id, &book.name);
                    }
                });
            ui.label(format!("{} songs", self.songs.len()));
        });

        let query = self.song_search.trim().to_lowercase();
        let filtered: Vec<Song> = self
            .songs
            .iter()
            .filter(|song| {
                self.songbook_filter == 0 || song.songbook_id == self.songbook_filter.to_string()
            })
            .filter(|song| {
                query.is_empty()
                    || song.title.to_lowercase().contains(&query)
                    || song.number.to_string().contains(&query)
                    || song.song_text.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();

        let style = ui.style().clone();
        ui.columns(2, |columns| {
            columns[0].set_width(320.0);
            egui::Frame::group(&style).show(&mut columns[0], |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("songs-list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if filtered.is_empty() {
                            ui.label(
                                RichText::new("No songs. Create a song or import a songbook.")
                                    .weak(),
                            );
                        }
                        for song in &filtered {
                            let selected = song.song_id == self.selected_song;
                            let label = format!("{:>3}. {}", song.number, song.title);
                            let response = ui.selectable_label(selected, label);
                            if response.clicked() {
                                self.selected_song = song.song_id;
                            }
                            if response.double_clicked() {
                                self.project_song(song, 0);
                            }
                        }
                    });
            });

            egui::Frame::group(&style).show(&mut columns[1], |ui| {
                let Some(song) = self.selected_song().cloned() else {
                    ui.label("Select a song to see its text.");
                    return;
                };
                ui.horizontal(|ui| {
                    ui.heading(format!("{}  {}", song.number, song.title));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑 Delete").clicked() {
                            match Song::delete(&self.db, song.song_id) {
                                Ok(()) => {
                                    self.reload_songs();
                                    self.set_status("Song deleted");
                                }
                                Err(err) => self.set_error("Could not delete song", err),
                            }
                        }
                        if ui.button("✏ Edit").clicked() {
                            self.song_editor = Some(SongEditor {
                                song: song.clone(),
                                is_new: false,
                            });
                        }
                        if ui.button("▶ Project").clicked() {
                            self.project_song(&song, 0);
                        }
                    });
                });
                ui.label(format!(
                    "Songbook: {}   Category: {}   Key/Tune: {}",
                    if song.songbook_name.is_empty() {
                        "—"
                    } else {
                        &song.songbook_name
                    },
                    song.category_name(),
                    if song.tune.is_empty() {
                        "—"
                    } else {
                        &song.tune
                    }
                ));
                if !song.words_by.is_empty() || !song.music_by.is_empty() {
                    ui.label(format!(
                        "Words: {}   Music: {}",
                        song.words_by, song.music_by
                    ));
                }
                ui.separator();

                let stanzas = song.text_list();
                paintable_song_preview(ui, &song, &stanzas, self);
            });
        });
    }

    fn new_song(&mut self) {
        if self.songbooks.is_empty() {
            match Songbook::add(&self.db, "Songs", "") {
                Ok(id) => {
                    self.songbook_filter = id;
                    self.reload_songs();
                }
                Err(err) => {
                    self.set_error("Could not create songbook", err);
                    return;
                }
            }
        }
        let songbook_id = self
            .songbooks
            .iter()
            .find(|b| b.id == self.songbook_filter)
            .or_else(|| self.songbooks.first())
            .map(|b| b.id)
            .unwrap_or(0);
        let number = Song::last_number(&self.db, &songbook_id.to_string()).unwrap_or(1);
        let mut song = Song::new(0);
        song.songbook_id = songbook_id.to_string();
        song.songbook_name = self
            .songbooks
            .iter()
            .find(|b| b.id == songbook_id)
            .map(|b| b.name.clone())
            .unwrap_or_default();
        song.number = number;
        song.font = FontSpec::new("Arial", 48);
        self.song_editor = Some(SongEditor { song, is_new: true });
    }

    pub fn song_editor_window(&mut self, ctx: &egui::Context) {
        let Some(mut editor) = self.song_editor.take() else {
            return;
        };
        let mut open = true;
        let mut save = false;
        let mut delete = false;
        let mut cancel = false;

        egui::Window::new(if editor.is_new {
            "New song"
        } else {
            "Edit song"
        })
        .open(&mut open)
        .default_width(760.0)
        .default_height(640.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("song-fields")
                    .num_columns(4)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Number");
                        ui.add(egui::DragValue::new(&mut editor.song.number).range(1..=9999));
                        ui.label("Songbook");
                        let book_name = self
                            .songbooks
                            .iter()
                            .find(|b| b.id.to_string() == editor.song.songbook_id)
                            .map(|b| b.name.clone())
                            .unwrap_or_else(|| "—".into());
                        egui::ComboBox::from_id_salt("song-editor-book")
                            .selected_text(book_name)
                            .show_ui(ui, |ui| {
                                for book in &self.songbooks {
                                    let id = book.id.to_string();
                                    ui.selectable_value(
                                        &mut editor.song.songbook_id,
                                        id,
                                        &book.name,
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Title");
                        ui.add(
                            egui::TextEdit::singleline(&mut editor.song.title).desired_width(300.0),
                        );
                        ui.label("Category");
                        egui::ComboBox::from_id_salt("song-category")
                            .selected_text(editor.song.category_name())
                            .show_ui(ui, |ui| {
                                for (index, name) in CATEGORIES.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut editor.song.category,
                                        index as i32,
                                        *name,
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Tune / key");
                        ui.add(
                            egui::TextEdit::singleline(&mut editor.song.tune).desired_width(200.0),
                        );
                        ui.label("Date");
                        ui.add(
                            egui::TextEdit::singleline(&mut editor.song.date)
                                .desired_width(120.0)
                                .hint_text("YYYY-MM-DD"),
                        );
                        ui.end_row();

                        ui.label("Words by");
                        ui.add(
                            egui::TextEdit::singleline(&mut editor.song.words_by)
                                .desired_width(200.0),
                        );
                        ui.label("Music by");
                        ui.add(
                            egui::TextEdit::singleline(&mut editor.song.music_by)
                                .desired_width(200.0),
                        );
                        ui.end_row();
                    });

                ui.add_space(6.0);
                ui.label("Song text (use Verse / Chorus / Slide markers):");
                ui.add(
                    egui::TextEdit::multiline(&mut editor.song.song_text)
                        .desired_rows(16)
                        .desired_width(f32::INFINITY)
                        .code_editor(),
                );
                ui.add_space(6.0);
                ui.label("Notes:");
                ui.add(
                    egui::TextEdit::multiline(&mut editor.song.notes)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );

                ui.add_space(6.0);
                ui.collapsing("Private display settings", |ui| {
                    ui.checkbox(
                        &mut editor.song.use_private_settings,
                        "Use private display settings for this song",
                    );
                    widgets::font_edit(ui, "Text font", &mut editor.song.font);
                    widgets::color_edit(ui, "Text color", &mut editor.song.color);
                    widgets::alignment_edit(
                        ui,
                        "Horizontal alignment",
                        &mut editor.song.alignment_h,
                        false,
                    );
                    widgets::alignment_edit(
                        ui,
                        "Vertical alignment",
                        &mut editor.song.alignment_v,
                        true,
                    );
                    widgets::font_edit(ui, "Info font", &mut editor.song.info_font);
                    widgets::color_edit(ui, "Info color", &mut editor.song.info_color);
                    widgets::font_edit(ui, "Ending font", &mut editor.song.ending_font);
                    widgets::color_edit(ui, "Ending color", &mut editor.song.ending_color);
                    widgets::background_edit(
                        ui,
                        &mut editor.song.use_background,
                        &mut editor.song.background_name,
                        &mut editor.song.background,
                    );
                });
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("💾 Save").clicked() {
                    save = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
                if !editor.is_new && ui.button("🗑 Delete").clicked() {
                    delete = true;
                }
            });
        });

        if cancel {
            open = false;
        }

        if delete {
            match Song::delete(&self.db, editor.song.song_id) {
                Ok(()) => {
                    self.reload_songs();
                    self.set_status("Song deleted");
                }
                Err(err) => self.set_error("Could not delete song", err),
            }
            return;
        }

        if save {
            match editor.song.save(&self.db) {
                Ok(()) => {
                    let id = editor.song.song_id;
                    self.selected_song = id;
                    self.reload_songs();
                    self.set_status("Song saved");
                }
                Err(err) => {
                    self.set_error("Could not save song", err);
                    self.song_editor = Some(editor);
                    return;
                }
            }
            return;
        }

        if open {
            self.song_editor = Some(editor);
        }
    }

    pub fn project_song(&mut self, song: &Song, start: usize) {
        let slides = build_song_slides(&self.theme, song);
        self.start_presentation(format!("{} — {}", song.number, song.title), slides, start);
    }

    pub fn add_song_to_schedule(&mut self, song: &Song) {
        self.schedule.push(ScheduleItem::from_song(song.clone()));
        self.selected_schedule = self.schedule.len().saturating_sub(1);
        self.set_status(format!("Added “{}” to schedule", song.title));
    }

    pub fn songbook_window(&mut self, ctx: &egui::Context) {
        if !self.show_songbooks {
            return;
        }
        let mut open = self.show_songbooks;
        egui::Window::new("Songbooks")
            .open(&mut open)
            .default_width(560.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for book in self.songbooks.clone() {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&book.name).strong());
                                ui.label(
                                    RichText::new(format!("({} songs)", self.count_songs(book.id)))
                                        .weak(),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("Delete").clicked() {
                                            match Songbook::delete(&self.db, book.id) {
                                                Ok(()) => {
                                                    self.reload_songs();
                                                    self.set_status("Songbook deleted");
                                                }
                                                Err(err) => {
                                                    self.set_error("Could not delete songbook", err)
                                                }
                                            }
                                        }
                                        if ui.small_button("Edit").clicked() {
                                            self.songbook_edit = Some((
                                                book.id,
                                                book.name.clone(),
                                                book.info.clone(),
                                            ));
                                        }
                                    },
                                );
                            });
                        }
                    });

                if let Some((id, name, info)) = self.songbook_edit.clone() {
                    ui.separator();
                    ui.label("Edit songbook");
                    let mut edit = (name, info);
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut edit.0);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Info");
                        ui.text_edit_multiline(&mut edit.1);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            match Songbook::update(&self.db, id, &edit.0, &edit.1) {
                                Ok(()) => {
                                    self.songbook_edit = None;
                                    self.reload_songs();
                                    self.set_status("Songbook saved");
                                }
                                Err(err) => self.set_error("Could not save songbook", err),
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.songbook_edit = None;
                        }
                    });
                }

                ui.separator();
                ui.label("New songbook");
                egui::Grid::new("new-songbook")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut self.new_songbook_name);
                        ui.end_row();
                        ui.label("Info");
                        ui.text_edit_singleline(&mut self.new_songbook_info);
                        ui.end_row();
                    });
                ui.horizontal(|ui| {
                    if ui.button("Add songbook").clicked() {
                        match Songbook::add(
                            &self.db,
                            &self.new_songbook_name,
                            &self.new_songbook_info,
                        ) {
                            Ok(_) => {
                                self.new_songbook_name.clear();
                                self.new_songbook_info.clear();
                                self.reload_songs();
                                self.set_status("Songbook created");
                            }
                            Err(err) => self.set_error("Could not create songbook", err),
                        }
                    }
                    if ui.button("Import...").clicked() {
                        self.import_songbook_dialog();
                    }
                    if ui.button("Export selected...").clicked() {
                        self.export_songbook_dialog();
                    }
                });
            });
        self.show_songbooks = open;
    }

    fn count_songs(&self, songbook_id: i64) -> usize {
        self.songs
            .iter()
            .filter(|song| song.songbook_id == songbook_id.to_string())
            .count()
    }

    pub fn import_songbook_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SoftProjector songbook", &["sps", "spb", "xml", "txt"])
            .add_filter("All files", &["*"])
            .pick_file()
        else {
            return;
        };
        match import_songbook(&self.db, &path) {
            Ok(id) => {
                self.reload_songs();
                self.songbook_filter = id;
                self.set_status(format!("Imported songbook {}", path.display()));
            }
            Err(err) => self.set_error("Could not import songbook", err),
        }
    }

    pub fn export_songbook_dialog(&mut self) {
        let id = if self.songbook_filter > 0 {
            self.songbook_filter
        } else {
            self.songbooks.first().map(|b| b.id).unwrap_or(0)
        };
        if id == 0 {
            self.set_error("Could not export songbook", "there are no songbooks");
            return;
        }
        let default_name = self
            .songbooks
            .iter()
            .find(|b| b.id == id)
            .map(|b| format!("{}.sps", b.name))
            .unwrap_or_else(|| "songbook.sps".into());
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SoftProjector songbook", &["sps"])
            .set_file_name(default_name)
            .save_file()
        else {
            return;
        };
        match export_songbook(&self.db, id, &path) {
            Ok(()) => self.set_status(format!("Exported songbook to {}", path.display())),
            Err(err) => self.set_error("Could not export songbook", err),
        }
    }
}

fn paintable_song_preview(ui: &mut egui::Ui, song: &Song, stanzas: &[String], app: &mut App) {
    let mut project_index: Option<usize> = None;
    egui::ScrollArea::vertical()
        .id_salt("song-preview")
        .show(ui, |ui| {
            for (index, stanza) in stanzas.iter().enumerate() {
                let first = stanza
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .take(70)
                    .collect::<String>();
                ui.horizontal(|ui| {
                    if ui.small_button(format!("▶ {}", index + 1)).clicked() {
                        project_index = Some(index);
                    }
                    ui.label(
                        RichText::new(format!("{first}…")).font(egui::FontId::proportional(15.0)),
                    );
                });
            }
            ui.separator();
            widgets::dark_preview(ui, 260.0, &song.song_text);
        });
    if let Some(index) = project_index {
        app.project_song(song, index);
        app.tab = Tab::Songs;
    }
}

/// Build the projection slides for a song using the current theme.
pub fn build_song_slides(theme: &Theme, song: &Song) -> Vec<Projected> {
    let sets = &theme.song;
    let mut style = TextStyle {
        font: sets.text_font.clone(),
        color: sets.text_color,
        align_h: sets.text_alignment_h,
        align_v: sets.text_alignment_v,
        screen_use: sets.screen_use,
        screen_position: sets.screen_position,
        shadow: sets.use_shadow,
        caption_font: sets.info_font.clone(),
        caption_color: sets.info_color,
        caption_align_h: 1,
        caption_at_top: sets.info_align == 0,
        info_font: sets.info_font.clone(),
        info_color: sets.info_color,
        info_at_top: sets.info_align == 0,
        ending_font: sets.ending_font.clone(),
        ending_color: sets.ending_color,
        ending_at_top: sets.ending_position == 0,
        background: if sets.use_background {
            sets.background.clone()
        } else {
            None
        },
    };

    if song.use_private_settings {
        style.font = song.font.clone();
        style.color = song.color;
        style.align_h = song.alignment_h;
        style.align_v = song.alignment_v;
        style.info_font = song.info_font.clone();
        style.info_color = song.info_color;
        style.ending_font = song.ending_font.clone();
        style.ending_color = song.ending_color;
        if song.use_background {
            style.background = song.background.clone();
        }
    }

    let stanzas = song.text_list();
    let count = stanzas.len();
    let mut slides = Vec::with_capacity(count);

    for index in 0..count {
        let stanza = song.stanza(index);
        let mut corner_left = String::new();
        if sets.show_stanza_title && !stanza.stanza_title.is_empty() {
            corner_left = stanza.stanza_title.clone();
        }
        let mut corner_right = String::new();
        if sets.show_song_number {
            corner_right.push_str(&song.number.to_string());
        }
        if sets.show_song_key && !song.tune.is_empty() {
            if !corner_right.is_empty() {
                corner_right.push_str("   ");
            }
            corner_right.push_str(&song.tune);
        }

        let ending = if stanza.is_last && sets.show_song_ending {
            song_ending(sets.ending_type, &song.words_by, &song.music_by)
        } else {
            String::new()
        };

        slides.push(Projected::Text(crate::project::TextSlide {
            text: stanza.stanza.clone(),
            caption: String::new(),
            corner_left,
            corner_right,
            ending,
            style: style.clone(),
            label: format!("{} — stanza {} of {}", song.title, index + 1, count),
        }));
    }

    slides
}
