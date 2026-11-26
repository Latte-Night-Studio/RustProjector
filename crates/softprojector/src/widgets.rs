//! Small UI helpers and platform integration.

use eframe::egui::{self, ComboBox, DragValue, FontId, RichText, Ui};
use sp_core::{FontSpec, RgbColor};

/// Colour picker bound to an [`RgbColor`].
pub fn color_edit(ui: &mut Ui, label: &str, color: &mut RgbColor) {
    ui.label(label);
    let mut rgb = [color.r, color.g, color.b];
    if ui.color_edit_button_srgb(&mut rgb).changed() {
        *color = RgbColor::new(rgb[0], rgb[1], rgb[2]);
    }
}

/// Font family + point size editor.
pub fn font_edit(ui: &mut Ui, label: &str, font: &mut FontSpec) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::TextEdit::singleline(&mut font.family)
                .desired_width(120.0)
                .hint_text("font"),
        );
        ui.add(
            DragValue::new(&mut font.point_size)
                .range(6..=300)
                .suffix(" pt"),
        );
        ui.checkbox(&mut font.italic, "italic");
        ui.checkbox(&mut font.underline, "underline");
    });
}

/// Horizontal/vertical alignment chooser.  `0` is left/top, `1` centre and `2`
/// right/bottom, matching the values stored by SoftProjector.
pub fn alignment_edit(ui: &mut Ui, label: &str, value: &mut i32, vertical: bool) {
    let names: [&str; 3] = if vertical {
        ["Top", "Center", "Bottom"]
    } else {
        ["Left", "Center", "Right"]
    };
    ui.horizontal(|ui| {
        ui.label(label);
        ComboBox::from_id_salt(("alignment", label, vertical))
            .selected_text(names[(*value).clamp(0, 2) as usize])
            .show_ui(ui, |ui| {
                for (index, name) in names.iter().enumerate() {
                    ui.selectable_value(value, index as i32, *name);
                }
            });
    });
}

/// Pick a background image and store its bytes.
pub fn background_edit(
    ui: &mut Ui,
    use_background: &mut bool,
    name: &mut String,
    bytes: &mut Option<Vec<u8>>,
) {
    ui.checkbox(use_background, "Use background image");
    ui.horizontal(|ui| {
        if ui.button("Choose background...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    "Images",
                    &["jpg", "jpeg", "png", "bmp", "gif", "webp", "tif", "tiff"],
                )
                .pick_file()
            {
                match std::fs::read(&path) {
                    Ok(data) => {
                        *bytes = Some(data);
                        *name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.to_string_lossy().to_string());
                    }
                    Err(err) => {
                        eprintln!("failed to read background: {err}");
                    }
                }
            }
        }
        if ui.button("Clear").clicked() {
            *bytes = None;
            name.clear();
        }
        ui.label(if name.is_empty() {
            "no image".to_string()
        } else {
            name.clone()
        });
    });
}

/// A text preview area with a dark background.
pub fn dark_preview(ui: &mut Ui, height: f32, text: &str) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(12, 12, 16))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_min_height(height);
            egui::ScrollArea::vertical()
                .max_height(height)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(text)
                            .font(FontId::proportional(15.0))
                            .color(egui::Color32::LIGHT_GRAY),
                    );
                });
        });
}

/// Monitor rectangles `(x, y, width, height)` ordered as Windows reports them.
#[cfg(windows)]
pub fn monitor_rects() -> Vec<(i32, i32, i32, i32)> {
    use windows_sys::Win32::Foundation::{LPARAM, RECT, TRUE};
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };

    unsafe extern "system" fn callback(
        monitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> windows_sys::Win32::Foundation::BOOL {
        let list = &mut *(data as *mut Vec<(i32, i32, i32, i32)>);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            let r = info.rcMonitor;
            list.push((r.left, r.top, r.right - r.left, r.bottom - r.top));
        }
        TRUE
    }

    let mut list = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(callback),
            &mut list as *mut _ as LPARAM,
        );
    }
    list
}

#[cfg(not(windows))]
pub fn monitor_rects() -> Vec<(i32, i32, i32, i32)> {
    Vec::new()
}

/// Open a file or URL with the operating system default application.
#[cfg(windows)]
pub fn open_external(path: &str) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let operation: Vec<u16> = "open\0".encode_utf16().collect();
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

#[cfg(not(windows))]
pub fn open_external(path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

/// Show a short single-line version of a multi-line text.
pub fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}
