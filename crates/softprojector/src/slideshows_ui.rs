//! Slideshows: image collections stored in the database.

use crate::app::{App, SlideshowDraft};
use crate::project::Projected;
use eframe::egui::{self, RichText};
use sp_core::slideshow::{load_slide_image, make_slide_item};
use sp_core::{ScheduleItem, SlideShow};

impl App {
    pub fn slideshows_tab(&mut self, ui: &mut egui::Ui) {
        let mut show = self.current_slideshow.take();
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.heading("Слайд-шоу");
            if ui.button("➕ Нове слайд-шоу...").clicked() {
                self.slideshow_draft = Some(SlideshowDraft {
                    name: "Нове слайд-шоу".into(),
                    info: String::new(),
                    files: Vec::new(),
                });
            }
            if ui.button("🖼 Додати зображення...").clicked() {
                if let Some(show) = show.as_mut() {
                    if let Some(paths) = rfd::FileDialog::new()
                        .add_filter(
                            "Зображення",
                            &["jpg", "jpeg", "png", "bmp", "gif", "webp", "tif", "tiff"],
                        )
                        .pick_files()
                    {
                        for path in paths {
                            let path_text = path.to_string_lossy().to_string();
                            match make_slide_item(&path_text, show.slides.len() as i32) {
                                Ok(item) => show.slides.push(item),
                                Err(err) => self.set_error("Не вдалося додати зображення", err),
                            }
                        }
                        changed = true;
                    }
                }
            }
            ui.separator();
            if ui.button("▶ Проєктувати").clicked() {
                if let Some(show) = show.as_ref() {
                    self.project_slideshow(show, 0);
                }
            }
            if ui.button("Додати до розкладу").clicked() {
                if let Some(show) = show.as_ref() {
                    self.schedule
                        .push(ScheduleItem::from_slideshow(show.clone()));
                    self.selected_schedule = self.schedule.len().saturating_sub(1);
                    self.set_status("Слайд-шоу додано до розкладу");
                }
            }
            if ui.button("🗑 Видалити").clicked() {
                if self.selected_slideshow > 0 {
                    match SlideShow::delete(&self.db, self.selected_slideshow) {
                        Ok(()) => {
                            self.current_slideshow = None;
                            self.slideshow_loaded_id = 0;
                            self.selected_slideshow = 0;
                            self.reload_slideshows();
                            self.set_status("Слайд-шоу видалено");
                        }
                        Err(err) => self.set_error("Не вдалося видалити слайд-шоу", err),
                    }
                }
            }
        });
        ui.separator();

        let style = ui.style().clone();
        ui.columns(2, |columns| {
            columns[0].set_width(260.0);
            egui::Frame::group(&style).show(&mut columns[0], |ui| {
                ui.heading("Усі слайд-шоу");
                let mut select_id = None;
                egui::ScrollArea::vertical()
                    .id_salt("slideshows-list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.slideshow_infos.is_empty() {
                            ui.label(RichText::new("Слайд-шоу ще немає.").weak());
                        }
                        for info in &self.slideshow_infos {
                            let selected = info.id == self.selected_slideshow;
                            if ui
                                .selectable_label(selected, format!("{}  ({})", info.name, info.id))
                                .clicked()
                            {
                                select_id = Some(info.id);
                            }
                        }
                    });
                if let Some(id) = select_id {
                    self.selected_slideshow = id;
                    self.slideshow_loaded_id = 0;
                }
                if let Some(show) = show.as_mut() {
                    ui.separator();
                    ui.label("Назва:");
                    ui.text_edit_singleline(&mut show.name);
                    ui.label("Інформація:");
                    ui.text_edit_singleline(&mut show.info);
                    if ui.button("💾 Зберегти дані").clicked() {
                        changed = true;
                    }
                }
            });

            egui::Frame::group(&style).show(&mut columns[1], |ui| {
                let Some(show) = show.as_mut() else {
                    ui.label("Виберіть слайд-шоу або створіть нове.");
                    return;
                };
                ui.horizontal(|ui| {
                    ui.heading(&show.name);
                    ui.label(RichText::new(format!("Слайдів: {}", show.slides.len())).weak());
                });
                if !show.info.is_empty() {
                    ui.label(RichText::new(&show.info).weak());
                }
                ui.separator();

                let thumbs: Vec<(usize, String, Option<Vec<u8>>)> = show
                    .slides
                    .iter()
                    .enumerate()
                    .map(|(index, slide)| {
                        (
                            index,
                            slide.name.clone(),
                            slide
                                .image_small
                                .clone()
                                .or_else(|| slide.image_preview.clone())
                                .or_else(|| slide.image.clone()),
                        )
                    })
                    .collect();

                let mut project_index: Option<usize> = None;
                let mut move_up: Option<usize> = None;
                let mut move_down: Option<usize> = None;
                let mut remove: Option<usize> = None;

                egui::ScrollArea::vertical()
                    .id_salt("slideshow-slides")
                    .show(ui, |ui| {
                        egui::Grid::new("slides-grid")
                            .num_columns(2)
                            .spacing([12.0, 10.0])
                            .show(ui, |ui| {
                                for (index, name, thumb) in &thumbs {
                                    ui.vertical(|ui| {
                                        if let Some(bytes) = thumb {
                                            if let Some(texture) =
                                                self.images.texture(ui.ctx(), "slide-thumb", bytes)
                                            {
                                                let size = texture.size_vec2();
                                                let scale = (180.0 / size.x).min(120.0 / size.y);
                                                ui.image((texture.id(), size * scale.max(0.01)));
                                            }
                                        } else {
                                            ui.allocate_space(egui::vec2(180.0, 110.0));
                                        }
                                        ui.label(
                                            RichText::new(format!("{}. {}", index + 1, name))
                                                .font(egui::FontId::proportional(12.0)),
                                        );
                                        ui.horizontal(|ui| {
                                            if ui.small_button("▶").clicked() {
                                                project_index = Some(*index);
                                            }
                                            if ui.small_button("↑").clicked() {
                                                move_up = Some(*index);
                                            }
                                            if ui.small_button("↓").clicked() {
                                                move_down = Some(*index);
                                            }
                                            if ui.small_button("✖").clicked() {
                                                remove = Some(*index);
                                            }
                                        });
                                    });
                                    if index % 3 == 2 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });

                if let Some(index) = project_index {
                    self.project_slideshow(show, index);
                }
                if let Some(index) = move_up {
                    if index > 0 {
                        show.slides.swap(index, index - 1);
                        changed = true;
                    }
                }
                if let Some(index) = move_down {
                    if index + 1 < show.slides.len() {
                        show.slides.swap(index, index + 1);
                        changed = true;
                    }
                }
                if let Some(index) = remove {
                    show.slides.remove(index);
                    changed = true;
                }

                if show.slides.is_empty() {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(
                            "Це слайд-шоу порожнє. Скористайтеся «Додати зображення...», щоб додати картинки.",
                        )
                        .weak(),
                    );
                }
            });
        });

        if let Some(mut show) = show {
            if changed {
                match show.save(&self.db) {
                    Ok(()) => {
                        self.reload_slideshows();
                        self.slideshow_loaded_id = show.id;
                        self.set_status("Слайд-шоу збережено");
                    }
                    Err(err) => self.set_error("Не вдалося зберегти слайд-шоу", err),
                }
            }
            self.current_slideshow = Some(show);
        }
    }

    pub fn project_slideshow(&mut self, show: &SlideShow, index: usize) {
        let slides: Vec<Projected> = show
            .slides
            .iter()
            .map(|slide| {
                if let Some(bytes) = load_slide_image(slide) {
                    Projected::Image {
                        bytes,
                        label: slide.name.clone(),
                    }
                } else {
                    Projected::Blank {
                        label: slide.name.clone(),
                    }
                }
            })
            .collect();
        self.start_presentation(format!("Слайд-шоу — {}", show.name), slides, index);
    }

    pub fn slideshow_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.slideshow_draft.take() else {
            return;
        };
        let mut open = true;
        let mut create = false;
        let mut cancel = false;
        egui::Window::new("Нове слайд-шоу")
            .open(&mut open)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Назва:");
                    ui.text_edit_singleline(&mut draft.name);
                });
                ui.horizontal(|ui| {
                    ui.label("Інформація:");
                    ui.text_edit_singleline(&mut draft.info);
                });
                ui.separator();
                ui.label(format!("Вибрано зображень: {}", draft.files.len()));
                ui.horizontal(|ui| {
                    if ui.button("Додати зображення...").clicked() {
                        if let Some(paths) = rfd::FileDialog::new()
                            .add_filter(
                                "Зображення",
                                &["jpg", "jpeg", "png", "bmp", "gif", "webp", "tif", "tiff"],
                            )
                            .pick_files()
                        {
                            for path in paths {
                                draft.files.push(path.to_string_lossy().to_string());
                            }
                        }
                    }
                    if ui.button("Очистити").clicked() {
                        draft.files.clear();
                    }
                });
                egui::ScrollArea::vertical()
                    .id_salt("draft-files")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for file in &draft.files {
                            ui.label(RichText::new(file).font(egui::FontId::proportional(12.0)));
                        }
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Створити").clicked() {
                        create = true;
                    }
                    if ui.button("Скасувати").clicked() {
                        cancel = true;
                    }
                });
            });

        if cancel {
            open = false;
        }

        if create {
            match SlideShow::create_from_files(&self.db, &draft.name, &draft.info, &draft.files) {
                Ok(show) => {
                    self.selected_slideshow = show.id;
                    self.slideshow_loaded_id = 0;
                    self.reload_slideshows();
                    self.set_status("Слайд-шоу створено");
                }
                Err(err) => self.set_error("Не вдалося створити слайд-шоу", err),
            }
        } else if open {
            self.slideshow_draft = Some(draft);
        }
    }
}
