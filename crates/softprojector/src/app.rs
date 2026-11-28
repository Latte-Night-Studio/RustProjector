//! Application shell: shared state, menus, tabs and the projector viewport.

use crate::project::{ImageCache, Presentation, Projected};
use crate::songs_ui;
use crate::widgets;
use eframe::egui::{self, Key, ViewportBuilder, ViewportCommand, ViewportId};
use sp_core::theme::ThemeInfo;
use sp_core::{
    Announcement, BibleSearch, BibleSession, BibleVersion, Database, MediaItem, ScheduleItem,
    Settings, SlideShow, SlideShowInfo, Song, Songbook, Theme,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Songs,
    Bible,
    Announcements,
    Slideshows,
    Media,
    Schedule,
    Themes,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeSection {
    Bible,
    Song,
    Announce,
    Passive,
}

pub struct SlideshowDraft {
    pub name: String,
    pub info: String,
    pub files: Vec<String>,
}

pub struct App {
    pub db: Database,
    pub settings: Settings,
    pub theme_infos: Vec<ThemeInfo>,
    pub theme: Theme,
    pub tab: Tab,

    pub status: String,
    pub error: Option<String>,
    pub about_open: bool,

    pub songs: Vec<Song>,
    pub songbooks: Vec<Songbook>,
    pub song_search: String,
    pub songbook_filter: i64,
    pub selected_song: i64,
    pub song_editor: Option<songs_ui::SongEditor>,
    pub show_songbooks: bool,
    pub new_songbook_name: String,
    pub new_songbook_info: String,
    pub songbook_edit: Option<(i64, String, String)>,

    pub bible_versions: Vec<BibleVersion>,
    pub bible_version_id: i64,
    pub bible_session: Option<BibleSession>,
    pub bible_book: i32,
    pub bible_chapter: i32,
    pub bible_selection: Vec<String>,
    pub bible_search: String,
    pub bible_all_words: bool,
    pub bible_scope_current: bool,
    pub bible_results: Vec<BibleSearch>,
    pub bible_extra_sessions: std::collections::HashMap<i64, BibleSession>,
    pub show_bible_manager: bool,

    pub announcements: Vec<Announcement>,
    pub announcement_editor: Option<Announcement>,

    pub slideshow_infos: Vec<SlideShowInfo>,
    pub selected_slideshow: i64,
    pub current_slideshow: Option<SlideShow>,
    pub slideshow_loaded_id: i64,
    pub slideshow_draft: Option<SlideshowDraft>,

    pub media: Vec<MediaItem>,
    pub selected_media: usize,

    pub schedule: Vec<ScheduleItem>,
    pub selected_schedule: usize,

    pub theme_display: i32,
    pub theme_section: ThemeSection,

    pub presentation: Presentation,
    pub projector_open: bool,
    pub projector_fullscreen: bool,
    pub last_video: Option<(String, usize, String)>,
    pub images: ImageCache,
    pub monitors: Vec<(i32, i32, i32, i32)>,
    pub update: crate::updater::UpdateState,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> anyhow::Result<Self> {
        install_fonts(&cc.egui_ctx);

        let db = Database::open_default()?;
        let mut settings = Settings::load(&db)?;
        let theme_id = if settings.general.current_theme_id > 0 {
            settings.general.current_theme_id
        } else {
            Theme::ensure_default(&db)?
        };
        let theme = Theme::load(&db, theme_id)?;
        settings.general.current_theme_id = theme.theme_id;

        let mut app = App {
            db,
            settings,
            theme_infos: Vec::new(),
            theme,
            tab: Tab::Songs,
            status: String::new(),
            error: None,
            about_open: false,
            songs: Vec::new(),
            songbooks: Vec::new(),
            song_search: String::new(),
            songbook_filter: 0,
            selected_song: 0,
            song_editor: None,
            show_songbooks: false,
            new_songbook_name: String::new(),
            new_songbook_info: String::new(),
            songbook_edit: None,
            bible_versions: Vec::new(),
            bible_version_id: 0,
            bible_session: None,
            bible_book: 0,
            bible_chapter: 1,
            bible_selection: Vec::new(),
            bible_search: String::new(),
            bible_all_words: false,
            bible_scope_current: false,
            bible_results: Vec::new(),
            bible_extra_sessions: std::collections::HashMap::new(),
            show_bible_manager: false,
            announcements: Vec::new(),
            announcement_editor: None,
            slideshow_infos: Vec::new(),
            selected_slideshow: 0,
            current_slideshow: None,
            slideshow_loaded_id: 0,
            slideshow_draft: None,
            media: Vec::new(),
            selected_media: 0,
            schedule: Vec::new(),
            selected_schedule: 0,
            theme_display: 1,
            theme_section: ThemeSection::Bible,
            presentation: Presentation::default(),
            projector_open: false,
            projector_fullscreen: true,
            last_video: None,
            images: ImageCache::default(),
            monitors: widgets::monitor_rects(),
            update: crate::updater::UpdateState::default(),
        };
        app.reload_all();
        app.update.check_at_startup();
        Ok(app)
    }

    pub fn reload_all(&mut self) {
        self.reload_songs();
        self.reload_announcements();
        self.reload_slideshows();
        self.reload_media();
        self.reload_themes();
        self.reload_bible_versions();
        if self.selected_song == 0 {
            self.selected_song = self.songs.first().map(|s| s.song_id).unwrap_or(0);
        }
        if self.selected_announcement_id() == 0 {
            self.announcement_editor = self.announcements.first().cloned();
        }
        if self.selected_slideshow == 0 {
            self.selected_slideshow = self.slideshow_infos.first().map(|s| s.id).unwrap_or(0);
        }
        self.ensure_slideshow_loaded();
        self.ensure_bible_session();
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.error = None;
    }

    pub fn set_error(&mut self, context: &str, error: impl std::fmt::Display) {
        self.error = Some(format!("{context}: {error}"));
    }

    pub fn reload_songs(&mut self) {
        match Song::load_all(&self.db) {
            Ok(songs) => {
                self.songs = songs;
                if !self.songs.iter().any(|s| s.song_id == self.selected_song) {
                    self.selected_song = self.songs.first().map(|s| s.song_id).unwrap_or(0);
                }
            }
            Err(err) => self.set_error("Не вдалося завантажити пісні", err),
        }
        match Songbook::load_all(&self.db) {
            Ok(books) => self.songbooks = books,
            Err(err) => self.set_error("Не вдалося завантажити збірники", err),
        }
    }

    pub fn reload_announcements(&mut self) {
        match Announcement::load_all(&self.db) {
            Ok(list) => self.announcements = list,
            Err(err) => self.set_error("Не вдалося завантажити оголошення", err),
        }
    }

    pub fn reload_slideshows(&mut self) {
        match SlideShow::load_all(&self.db) {
            Ok(list) => self.slideshow_infos = list,
            Err(err) => self.set_error("Не вдалося завантажити слайд-шоу", err),
        }
    }

    pub fn reload_media(&mut self) {
        match MediaItem::load_all(&self.db) {
            Ok(list) => self.media = list,
            Err(err) => self.set_error("Не вдалося завантажити медіа", err),
        }
        self.selected_media = self.selected_media.min(self.media.len().saturating_sub(1));
    }

    pub fn reload_themes(&mut self) {
        match Theme::list(&self.db) {
            Ok(list) => self.theme_infos = list,
            Err(err) => self.set_error("Не вдалося завантажити теми", err),
        }
    }

    pub fn reload_bible_versions(&mut self) {
        match BibleVersion::load_all(&self.db) {
            Ok(list) => {
                self.bible_versions = list;
                if !self
                    .bible_versions
                    .iter()
                    .any(|v| v.id == self.bible_version_id)
                {
                    self.bible_version_id = self
                        .settings
                        .bible_sets
                        .primary_bible
                        .parse::<i64>()
                        .ok()
                        .filter(|id| self.bible_versions.iter().any(|v| v.id == *id))
                        .or_else(|| self.bible_versions.first().map(|v| v.id))
                        .unwrap_or(0);
                }
            }
            Err(err) => self.set_error("Не вдалося завантажити версії Біблії", err),
        }
    }

    pub fn ensure_bible_session(&mut self) {
        if self.bible_version_id <= 0 {
            self.bible_session = None;
            return;
        }
        if self
            .bible_session
            .as_ref()
            .is_some_and(|s| s.version.id == self.bible_version_id)
        {
            return;
        }
        match BibleSession::load(&self.db, self.bible_version_id) {
            Ok(Some(session)) => {
                self.bible_book = session.books.first().map(|b| b.book_id).unwrap_or(0);
                self.bible_chapter = 1;
                self.bible_selection.clear();
                self.bible_results.clear();
                self.bible_session = Some(session);
            }
            Ok(None) => self.bible_session = None,
            Err(err) => self.set_error("Не вдалося завантажити Біблію", err),
        }
    }

    pub fn ensure_slideshow_loaded(&mut self) {
        if self.selected_slideshow <= 0 {
            self.current_slideshow = None;
            self.slideshow_loaded_id = 0;
            return;
        }
        if self.slideshow_loaded_id == self.selected_slideshow {
            return;
        }
        match SlideShow::load(&self.db, self.selected_slideshow) {
            Ok(show) => {
                self.current_slideshow = Some(show);
                self.slideshow_loaded_id = self.selected_slideshow;
            }
            Err(err) => {
                self.current_slideshow = None;
                self.slideshow_loaded_id = 0;
                self.set_error("Не вдалося завантажити слайд-шоу", err);
            }
        }
    }

    pub fn selected_announcement_id(&self) -> i64 {
        self.announcement_editor.as_ref().map(|a| a.id).unwrap_or(0)
    }

    pub fn selected_song(&self) -> Option<&Song> {
        self.songs.iter().find(|s| s.song_id == self.selected_song)
    }

    pub fn load_theme(&mut self, theme_id: i64) {
        match Theme::load(&self.db, theme_id) {
            Ok(theme) => {
                self.theme = theme;
                self.settings.general.current_theme_id = self.theme.theme_id;
                if let Err(err) = self.settings.save(&self.db) {
                    self.set_error("Не вдалося зберегти налаштування", err);
                }
                self.images.clear();
            }
            Err(err) => self.set_error("Не вдалося завантажити тему", err),
        }
    }

    pub fn start_presentation(
        &mut self,
        source: impl Into<String>,
        slides: Vec<Projected>,
        index: usize,
    ) {
        if slides.is_empty() {
            self.set_error("Нічого проєктувати", "презентація не містить слайдів");
            return;
        }
        let source = source.into();
        self.status = format!("Проєктується {source} (слайдів: {})", slides.len());
        self.presentation = Presentation::new(source, slides);
        self.presentation.index = index.min(self.presentation.slides.len() - 1);
        self.presentation.black = false;
        self.presentation.auto_next = false;
        self.presentation.loop_slides = false;
        self.presentation.slide_seconds = 0.0;
        self.presentation.slide_shown_at = -1.0;
        self.projector_open = true;
        self.last_video = None;
    }

    fn poll_auto_advance(&mut self, now: f64) {
        let presentation = &mut self.presentation;
        if !presentation.auto_next
            || presentation.black
            || presentation.slides.is_empty()
            || presentation.slide_seconds <= 0.0
        {
            return;
        }
        if presentation.slide_shown_at < 0.0 {
            presentation.slide_shown_at = now;
            return;
        }
        if now - presentation.slide_shown_at >= presentation.slide_seconds as f64 {
            if !presentation.next() {
                if presentation.loop_slides {
                    presentation.index = 0;
                    presentation.slide_shown_at = now;
                } else {
                    presentation.auto_next = false;
                }
            }
        }
    }

    pub fn stop_projection(&mut self) {
        self.presentation = Presentation::default();
        self.last_video = None;
    }

    fn poll_video(&mut self) {
        if !self.projector_open || self.presentation.black {
            return;
        }
        let Some(Projected::Video { path, .. }) = self.presentation.current() else {
            return;
        };
        let key = (
            self.presentation.source.clone(),
            self.presentation.index,
            path.clone(),
        );
        if self.last_video.as_ref() != Some(&key) {
            self.last_video = Some(key);
            widgets::open_external(path);
        }
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Файл", |ui| {
                    if ui.button("Імпортувати збірник...").clicked() {
                        ui.close_menu();
                        self.import_songbook_dialog();
                    }
                    if ui.button("Експортувати поточний збірник...").clicked()
                    {
                        ui.close_menu();
                        self.export_songbook_dialog();
                    }
                    ui.separator();
                    if ui.button("Імпортувати Біблію...").clicked() {
                        ui.close_menu();
                        self.import_bible_dialog();
                    }
                    ui.separator();
                    if ui.button("Імпортувати теми...").clicked() {
                        ui.close_menu();
                        self.import_themes_dialog();
                    }
                    if ui.button("Експортувати всі теми...").clicked() {
                        ui.close_menu();
                        self.export_themes_dialog();
                    }
                    ui.separator();
                    if ui.button("Відкрити розклад...").clicked() {
                        ui.close_menu();
                        self.open_schedule_dialog();
                    }
                    if ui.button("Зберегти розклад...").clicked() {
                        ui.close_menu();
                        self.save_schedule_dialog();
                    }
                    ui.separator();
                    if ui.button("Вихід").clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                });
                ui.menu_button("Довідка", |ui| {
                    if ui.button("Про SoftProjector").clicked() {
                        self.about_open = true;
                        ui.close_menu();
                    }
                });
                ui.separator();
                ui.label(egui::RichText::new("SoftProjector").strong());
                ui.label(format!("v{}", sp_core::APP_VERSION));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("БД: {}", self.db.path().display()));
                });
            });

            ui.horizontal(|ui| {
                for (tab, label) in [
                    (Tab::Songs, "Пісні"),
                    (Tab::Bible, "Біблія"),
                    (Tab::Announcements, "Оголошення"),
                    (Tab::Slideshows, "Слайд-шоу"),
                    (Tab::Media, "Медіа"),
                    (Tab::Schedule, "Розклад"),
                    (Tab::Themes, "Теми"),
                    (Tab::Settings, "Налаштування"),
                ] {
                    if ui.selectable_label(self.tab == tab, label).clicked() {
                        self.tab = tab;
                    }
                }
            });
        });

        egui::TopBottomPanel::top("projection_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(if self.projector_open {
                        "📺 Проєктор увімкнено"
                    } else {
                        "📺 Запустити проєктор"
                    })
                    .clicked()
                {
                    self.projector_open = !self.projector_open;
                    if !self.projector_open {
                        self.stop_projection();
                    }
                }
                if ui
                    .checkbox(&mut self.projector_fullscreen, "На весь екран")
                    .changed()
                {
                    ctx.send_viewport_cmd_to(
                        ViewportId::from_hash_of("softprojector-projector"),
                        ViewportCommand::Fullscreen(self.projector_fullscreen),
                    );
                }
                let mut black = self.presentation.black;
                if ui.checkbox(&mut black, "Чорний екран").changed() {
                    self.presentation.black = black;
                }
                ui.separator();
                if ui.button("⏮").on_hover_text("Попередній слайд").clicked() {
                    self.presentation.previous();
                }
                if ui.button("⏭").on_hover_text("Наступний слайд").clicked() {
                    self.presentation.next();
                }
                if ui.button("⏹").on_hover_text("Зупинити проєкцію").clicked() {
                    self.stop_projection();
                }
                ui.separator();
                if self.presentation.is_empty() {
                    ui.label("Нічого не проєктується");
                } else {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} — {}",
                            self.presentation.source,
                            self.presentation.counter()
                        ))
                        .strong(),
                    );
                    if let Some(current) = self.presentation.current() {
                        ui.label(egui::RichText::new(current.label()).weak());
                    }
                }
            });
        });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(error) = self.error.clone() {
                    ui.label(egui::RichText::new(error).color(egui::Color32::LIGHT_RED));
                    if ui.small_button("Закрити").clicked() {
                        self.error = None;
                    }
                } else {
                    ui.label(&self.status);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "Тема: {} | Дисплей {}",
                        self.theme.name, self.theme_display
                    ));
                });
            });
        });
    }

    fn update_bar(&mut self, ctx: &egui::Context) {
        if self.update.dismissed {
            return;
        }
        let visible = self.update.downloading
            || self.update.ready.is_some()
            || self.update.info.is_some()
            || self.update.error.is_some();
        if !visible {
            return;
        }

        let mut check = false;
        let mut download = false;
        let mut install = false;
        let mut dismiss = false;
        let mut open_release = false;

        egui::TopBottomPanel::bottom("update_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if self.update.downloading {
                    ui.spinner();
                    ui.label("Завантаження оновлення…");
                    return;
                }
                if self.update.ready.is_some() {
                    ui.label(
                        egui::RichText::new("Оновлення завантажено.")
                            .color(egui::Color32::LIGHT_GREEN),
                    );
                    if ui
                        .add_enabled(
                            self.update.can_install(),
                            egui::Button::new("Перезапустити та встановити"),
                        )
                        .clicked()
                    {
                        install = true;
                    }
                    if ui.button("Пізніше").clicked() {
                        dismiss = true;
                    }
                    return;
                }
                if let Some(error) = self.update.error.clone() {
                    ui.label(
                        egui::RichText::new(format!("Помилка оновлення: {error}"))
                            .color(egui::Color32::LIGHT_RED),
                    );
                    if ui.button("Повторити").clicked() {
                        if self.update.info.is_some() {
                            download = true;
                        } else {
                            check = true;
                        }
                    }
                    if ui.button("Закрити").clicked() {
                        dismiss = true;
                    }
                    return;
                }
                if let Some(info) = self.update.info.clone() {
                    ui.label(
                        egui::RichText::new(format!("Доступне оновлення v{}.", info.version))
                            .strong(),
                    );
                    if ui.button("Оновити").clicked() {
                        download = true;
                    }
                    if ui
                        .button("Що нового?")
                        .on_hover_text(&info.page_url)
                        .clicked()
                    {
                        open_release = true;
                    }
                    if ui.button("Пізніше").clicked() {
                        dismiss = true;
                    }
                    return;
                }
            });
        });

        if open_release {
            if let Some(info) = &self.update.info {
                widgets::open_external(&info.page_url);
            }
        }
        if check {
            self.update.check_now();
        }
        if download {
            self.update.download();
        }
        if dismiss {
            self.update.dismissed = true;
        }
        if install {
            if let Some(path) = self.update.ready.clone() {
                match crate::updater::launch_installer(&path) {
                    Ok(()) => {
                        self.projector_open = false;
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                    Err(error) => {
                        self.set_error("Не вдалося встановити оновлення", error);
                    }
                }
            }
        }
    }

    fn projector_window(&mut self, ctx: &egui::Context) {
        if !self.projector_open {
            return;
        }

        let monitor = self
            .monitors
            .get(self.settings.general.display_screen.max(0) as usize)
            .copied();
        let mut builder = ViewportBuilder::default()
            .with_title("Проєкція SoftProjector")
            .with_decorations(false)
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([320.0, 240.0]);
        if let Some((x, y, _, _)) = monitor {
            builder = builder.with_position(egui::pos2(x as f32, y as f32));
        }
        if self.projector_fullscreen {
            builder = builder.with_fullscreen(true);
        }
        if self.settings.general.display_is_on_top {
            builder = builder.with_always_on_top();
        }

        let presentation = &mut self.presentation;
        let images = &mut self.images;
        let mut open = true;

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("softprojector-projector"),
            builder,
            |ctx, _class| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    open = false;
                }
                if ctx.input(|i| i.key_pressed(Key::Escape)) {
                    open = false;
                }
                if ctx.input(|i| i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::Space)) {
                    presentation.next();
                }
                if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
                    presentation.previous();
                }
                if ctx.input(|i| i.key_pressed(Key::B)) {
                    presentation.black = !presentation.black;
                }

                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
                    .show(ctx, |ui| {
                        let rect = ui.max_rect();
                        crate::project::render_projection(
                            ui,
                            rect,
                            presentation.current(),
                            presentation.black,
                            images,
                        );
                    });

                if !open {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            },
        );

        if !open {
            self.projector_open = false;
            self.stop_projection();
        }
    }

    fn about_window(&mut self, ctx: &egui::Context) {
        if !self.about_open {
            return;
        }
        let mut open = self.about_open;
        egui::Window::new("Про SoftProjector")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("SoftProjector — Rust-версія");
                ui.label(format!("Версія {}", sp_core::APP_VERSION));
                ui.separator();
                ui.label("Rust-реалізація церковної програми проєкції SoftProjector 2.");
                ui.label("Схема бази даних версії 2, сумісна з spData.sqlite.");
                ui.label(format!("База даних: {}", self.db.path().display()));
                ui.label("Іконки: стандартні шрифти egui. Системний шрифт: Arial, якщо доступний.");
            });
        self.about_open = open;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_auto_advance(ctx.input(|i| i.time));
        self.poll_video();
        self.update.poll();
        if self.update.checking
            || self.update.downloading
            || (self.update.ready.is_some() && !self.update.can_install())
        {
            // Keep the spinner moving and switch the banner as soon as the
            // background work finishes, even when the mouse is idle.
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        self.ensure_bible_session();
        self.ensure_slideshow_loaded();

        self.top_bar(ctx);
        self.update_bar(ctx);
        self.status_bar(ctx);

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Songs => self.songs_tab(ui),
            Tab::Bible => self.bible_tab(ui),
            Tab::Announcements => self.announcements_tab(ui),
            Tab::Slideshows => self.slideshows_tab(ui),
            Tab::Media => self.media_tab(ui),
            Tab::Schedule => self.schedule_tab(ui),
            Tab::Themes => self.themes_tab(ui),
            Tab::Settings => self.settings_tab(ui),
        });

        self.songbook_window(ctx);
        self.song_editor_window(ctx);
        self.slideshow_dialog(ctx);
        self.bible_manager_window(ctx);
        self.about_window(ctx);
        self.projector_window(ctx);
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        "C:\\Windows\\Fonts\\arial.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\DejaVuSans.ttf",
    ];
    for candidate in candidates {
        if let Ok(data) = std::fs::read(candidate) {
            let name = "system-display-font".to_string();
            fonts.font_data.insert(
                name.clone(),
                std::sync::Arc::new(egui::FontData::from_owned(data)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, name);
            break;
        }
    }
    ctx.set_fonts(fonts);
}
