//! SoftProjector — desktop projection application (Rust port).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod announcements_ui;
mod app;
mod bible_ui;
mod media_ui;
mod project;
mod schedule_ui;
mod settings_ui;
mod slideshows_ui;
mod songs_ui;
mod themes_ui;
mod widgets;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SoftProjector")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([960.0, 600.0])
            .with_app_id("softprojector"),
        ..Default::default()
    };

    eframe::run_native(
        "SoftProjector",
        options,
        Box::new(|cc| {
            let app =
                app::App::new(cc).map_err(|err| -> Box<dyn std::error::Error + Send + Sync> {
                    Box::new(std::io::Error::other(format!(
                        "Could not open the SoftProjector database: {err}"
                    )))
                })?;
            Ok(Box::new(app))
        }),
    )
}
