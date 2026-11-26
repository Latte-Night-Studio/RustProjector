//! Media library: images and videos used during a service.

use crate::app::App;
use crate::project::Projected;
use crate::widgets;
use eframe::egui::{self, RichText};
use sp_core::media::{is_image_path, is_video_path};
use sp_core::{MediaItem, ScheduleItem};

impl App {
    pub fn media_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Media");
            if ui.button("➕ Add files...").clicked() {
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter(
                        "Media",
                        &[
                            "mp4", "m4v", "avi", "mkv", "wmv", "mov", "mpg", "mpeg", "webm", "flv",
                            "ts", "jpg", "jpeg", "png", "gif", "bmp", "webp", "tif", "tiff",
                        ],
                    )
                    .pick_files()
                {
                    for path in paths {
                        let text = path.to_string_lossy().to_string();
                        if let Err(err) = MediaItem::add(&self.db, &text) {
                            self.set_error("Could not add media", err);
                        }
                    }
                    self.reload_media();
                    self.set_status("Media files added");
                }
            }
            if ui.button("🔄 Refresh").clicked() {
                self.reload_media();
            }
            ui.separator();
            if let Some(item) = self.media.get(self.selected_media).cloned() {
                if ui.button("▶ Project").clicked() {
                    self.project_media(&item);
                }
                if ui.button("Add to schedule").clicked() {
                    self.schedule.push(ScheduleItem::from_media(item.clone()));
                    self.selected_schedule = self.schedule.len().saturating_sub(1);
                    self.set_status("Media added to schedule");
                }
                if ui.button("Open in player").clicked() {
                    if item.exists() {
                        widgets::open_external(&item.long_path);
                    } else {
                        self.set_error("Could not open media", "the file does not exist");
                    }
                }
            }
        });
        ui.separator();

        let style = ui.style().clone();
        ui.columns(2, |columns| {
            columns[0].set_width(340.0);
            egui::Frame::group(&style).show(&mut columns[0], |ui| {
                ui.heading("Library");
                let mut selected = self.selected_media;
                egui::ScrollArea::vertical()
                    .id_salt("media-list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.media.is_empty() {
                            ui.label(
                                RichText::new("No media yet. Add images or videos to the library.")
                                    .weak(),
                            );
                        }
                        for (index, item) in self.media.iter().enumerate() {
                            let selected_label = index == selected;
                            let icon = if is_video_path(&item.long_path) {
                                "🎞"
                            } else {
                                "🖼"
                            };
                            let missing = if item.exists() { "" } else { "  (missing)" };
                            if ui
                                .selectable_label(
                                    selected_label,
                                    format!("{icon} {}{missing}", item.short_path),
                                )
                                .clicked()
                            {
                                selected = index;
                            }
                        }
                    });
                self.selected_media = selected;
            });

            egui::Frame::group(&style).show(&mut columns[1], |ui| {
                let Some(item) = self.media.get(self.selected_media).cloned() else {
                    ui.label("Select a media item to see its details.");
                    return;
                };
                ui.heading(&item.short_path);
                ui.label(RichText::new(&item.long_path).weak());
                ui.label(format!(
                    "Type: {}",
                    if is_video_path(&item.long_path) {
                        "Video"
                    } else if is_image_path(&item.long_path) {
                        "Image"
                    } else {
                        "Unknown"
                    }
                ));
                ui.label(format!(
                    "Status: {}",
                    if item.exists() {
                        "Available"
                    } else {
                        "File is missing"
                    }
                ));
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("▶ Project").clicked() {
                        self.project_media(&item);
                    }
                    if ui.button("▶ Play externally").clicked() {
                        widgets::open_external(&item.long_path);
                    }
                    if ui.button("🗑 Remove from library").clicked() {
                        match MediaItem::delete_path(&self.db, &item.long_path) {
                            Ok(()) => {
                                self.reload_media();
                                self.set_status("Media removed from library");
                            }
                            Err(err) => self.set_error("Could not remove media", err),
                        }
                    }
                });

                if is_image_path(&item.long_path) {
                    if let Ok(bytes) = std::fs::read(&item.long_path) {
                        if let Some(texture) =
                            self.images.texture(ui.ctx(), "media-preview", &bytes)
                        {
                            let size = texture.size_vec2();
                            let scale = (ui.available_width() / size.x)
                                .min((ui.available_height() - 20.0) / size.y)
                                .max(0.01);
                            ui.add_space(8.0);
                            ui.image((texture.id(), size * scale));
                        }
                    }
                } else {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("Videos are opened in the system's default media player.")
                            .weak(),
                    );
                }
            });
        });
    }

    pub fn project_media(&mut self, item: &MediaItem) {
        if !item.exists() {
            self.set_error("Cannot project media", "the file does not exist");
            return;
        }
        let slide = if is_video_path(&item.long_path) {
            Projected::Video {
                path: item.long_path.clone(),
                label: item.short_path.clone(),
            }
        } else if is_image_path(&item.long_path) {
            match std::fs::read(&item.long_path) {
                Ok(bytes) => Projected::Image {
                    bytes,
                    label: item.short_path.clone(),
                },
                Err(err) => {
                    self.set_error("Could not read image", err);
                    return;
                }
            }
        } else {
            self.set_error("Cannot project media", "unsupported file type");
            return;
        };
        self.start_presentation(format!("Media — {}", item.short_path), vec![slide], 0);
    }
}
