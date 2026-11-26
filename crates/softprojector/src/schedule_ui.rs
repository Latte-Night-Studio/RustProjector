//! Service schedule: ordering, saving and projecting.

use crate::app::App;
use eframe::egui::{self, RichText};
use sp_core::media::is_video_path;
use sp_core::schedule::{open_schedule, save_schedule};
use sp_core::ScheduleKind;

impl App {
    pub fn schedule_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Schedule");
            if ui.button("📂 Open...").clicked() {
                self.open_schedule_dialog();
            }
            if ui.button("💾 Save as...").clicked() {
                self.save_schedule_dialog();
            }
            if ui.button("🗑 Clear").clicked() {
                self.schedule.clear();
                self.selected_schedule = 0;
                self.set_status("Schedule cleared");
            }
            ui.separator();
            if ui.button("▶ Project selected").clicked() {
                self.project_schedule_item(self.selected_schedule);
            }
            ui.label(format!("{} item(s)", self.schedule.len()));
        });
        ui.separator();

        if self.schedule.is_empty() {
            ui.label(
                RichText::new(
                    "The schedule is empty. Add songs, Bible verses, announcements, slideshows or media from their tabs.",
                )
                .weak(),
            );
            return;
        }

        let mut move_up: Option<usize> = None;
        let mut move_down: Option<usize> = None;
        let mut remove: Option<usize> = None;
        let mut project: Option<usize> = None;
        let mut select: Option<usize> = None;
        let selected = self.selected_schedule;

        egui::ScrollArea::vertical()
            .id_salt("schedule-list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, item) in self.schedule.iter().enumerate() {
                    let kind = match &item.kind {
                        ScheduleKind::Bible(_) => "Bible",
                        ScheduleKind::Song(_) => "Song",
                        ScheduleKind::SlideShow(_) => "Slideshow",
                        ScheduleKind::Media(media) => {
                            if is_video_path(&media.long_path) {
                                "Video"
                            } else {
                                "Image"
                            }
                        }
                        ScheduleKind::Announce(_) => "Announcement",
                    };
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                index == selected,
                                format!("{:>2}. [{}] {}", index + 1, kind, item.name),
                            )
                            .clicked()
                        {
                            select = Some(index);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("✖").on_hover_text("Remove").clicked() {
                                remove = Some(index);
                            }
                            if ui.small_button("▼").on_hover_text("Move down").clicked() {
                                move_down = Some(index);
                            }
                            if ui.small_button("▲").on_hover_text("Move up").clicked() {
                                move_up = Some(index);
                            }
                            if ui.small_button("▶").on_hover_text("Project").clicked() {
                                project = Some(index);
                            }
                        });
                    });
                }
            });

        if let Some(index) = select {
            self.selected_schedule = index;
        }
        if let Some(index) = project {
            self.selected_schedule = index;
            self.project_schedule_item(index);
        }
        if let Some(index) = move_up {
            if index > 0 {
                self.schedule.swap(index, index - 1);
                self.selected_schedule = index - 1;
            }
        }
        if let Some(index) = move_down {
            if index + 1 < self.schedule.len() {
                self.schedule.swap(index, index + 1);
                self.selected_schedule = index + 1;
            }
        }
        if let Some(index) = remove {
            self.schedule.remove(index);
            self.selected_schedule = self
                .selected_schedule
                .min(self.schedule.len().saturating_sub(1));
        }
    }

    pub fn project_schedule_item(&mut self, index: usize) {
        let Some(item) = self.schedule.get(index).cloned() else {
            return;
        };
        let name = item.name.clone();
        match item.kind {
            ScheduleKind::Song(song) => self.project_song(&song, 0),
            ScheduleKind::SlideShow(show) => self.project_slideshow(&show, 0),
            ScheduleKind::Announce(announcement) => self.project_announcement(&announcement, 0),
            ScheduleKind::Media(media) => self.project_media(&media),
            ScheduleKind::Bible(history) => {
                self.project_bible_ids(&history.verse_ids, &name);
            }
        }
    }

    pub fn open_schedule_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SoftProjector schedule", &["spsc"])
            .add_filter("All files", &["*"])
            .pick_file()
        else {
            return;
        };
        match open_schedule(&path) {
            Ok(items) => {
                self.schedule = items;
                self.selected_schedule = 0;
                self.set_status(format!(
                    "Opened schedule {} ({} items)",
                    path.display(),
                    self.schedule.len()
                ));
            }
            Err(err) => self.set_error("Could not open schedule", err),
        }
    }

    pub fn save_schedule_dialog(&mut self) {
        if self.schedule.is_empty() {
            self.set_error("Could not save schedule", "the schedule is empty");
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SoftProjector schedule", &["spsc"])
            .set_file_name("service.spsc")
            .save_file()
        else {
            return;
        };
        let mut items = self.schedule.clone();
        match save_schedule(&path, &mut items) {
            Ok(()) => {
                self.schedule = items;
                self.set_status(format!("Saved schedule to {}", path.display()));
            }
            Err(err) => self.set_error("Could not save schedule", err),
        }
    }
}
