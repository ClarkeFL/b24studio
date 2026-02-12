use std::collections::VecDeque;
use eframe::egui;
use uuid::Uuid;
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::codec;

// ── Shared UI constants ─────────────────────────────────────────
pub const PAGE_HEADER_SIZE: f32 = 22.0;
pub const SECTION_HEADER_SIZE: f32 = 17.0;
pub const SECTION_HEADER_COLOR: egui::Color32 = egui::Color32::from_rgb(130, 170, 220);

pub const BTN_HEIGHT_HEADER: f32 = 32.0;   // Header row buttons
pub const BTN_HEIGHT_INLINE: f32 = 28.0;   // Small / inline buttons
pub const BTN_HEIGHT_PRIMARY: f32 = 36.0;  // Primary action buttons

pub const SPINNER_SIZE: f32 = 16.0;

pub const COLOR_BTN_GREEN: egui::Color32 = egui::Color32::from_rgb(40, 120, 60);
pub const COLOR_BTN_RED: egui::Color32 = egui::Color32::from_rgb(180, 50, 50);

pub const COLOR_ERROR: egui::Color32 = egui::Color32::from_rgb(255, 100, 100);
pub const COLOR_WARNING: egui::Color32 = egui::Color32::from_rgb(255, 180, 80);
pub const COLOR_SUCCESS: egui::Color32 = egui::Color32::from_rgb(80, 200, 80);

// ── Theme-aware colors ──────────────────────────────────────────

pub fn section_header_color(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(130, 170, 220) }
    else    { egui::Color32::from_rgb(40, 90, 160) }
}

pub fn link_color(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(100, 180, 255) }
    else    { egui::Color32::from_rgb(0, 100, 200) }
}

pub fn help_icon_color(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(120, 160, 210) }
    else    { egui::Color32::from_rgb(60, 110, 170) }
}

pub fn subtle_bg(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(35, 40, 55) }
    else    { egui::Color32::from_rgb(235, 238, 242) }
}

pub fn subtle_bg_selected(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(40, 55, 80) }
    else    { egui::Color32::from_rgb(215, 228, 245) }
}

pub fn subtle_border(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(80, 140, 220) }
    else    { egui::Color32::from_rgb(100, 140, 200) }
}

pub fn subtle_border_dim(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(60, 60, 70) }
    else    { egui::Color32::from_rgb(190, 195, 205) }
}

pub fn muted_text(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(140, 140, 140) }
    else    { egui::Color32::from_rgb(100, 100, 100) }
}

pub fn bright_info(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(160, 200, 255) }
    else    { egui::Color32::from_rgb(20, 80, 170) }
}

/// Consistent page title: 22pt bold
pub fn page_header(ui: &mut egui::Ui, title: &str) {
    ui.label(egui::RichText::new(title).size(PAGE_HEADER_SIZE).strong());
}

/// Consistent section header: 17pt bold blue, with separator
pub fn section_header(ui: &mut egui::Ui, title: &str) {
    let dark = ui.visuals().dark_mode;
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(title)
            .size(SECTION_HEADER_SIZE)
            .strong()
            .color(section_header_color(dark)),
    );
    ui.separator();
    ui.add_space(4.0);
}

/// Standalone help icon that reliably shows a tooltip on hover.
/// Renders as a small "(?)" badge with guaranteed minimum interaction area.
pub fn help_icon(ui: &mut egui::Ui, tooltip: &str) {
    let dark = ui.visuals().dark_mode;
    let icon = egui::RichText::new("(?)")
        .size(13.0)
        .color(help_icon_color(dark));
    let resp = ui.add_sized(
        [24.0, 20.0],
        egui::Label::new(icon).sense(egui::Sense::hover()),
    );
    resp.on_hover_text(tooltip);
}

/// A field label with an optional help icon that shows a tooltip on hover.
/// Uses a fixed-width label plus the reliable help_icon widget.
/// Pass `None` for help_text to show a plain label with no icon.
pub fn field_label_with_help(ui: &mut egui::Ui, label: &str, help_text: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(15.0));
        if let Some(help) = help_text {
            help_icon(ui, help);
        }
    });
}

/// Render a register row: Label | Current Value | [Edit Field] | Read | [Write]
pub fn register_row_f32(
    ui: &mut egui::Ui,
    label: &str,
    current: Option<f32>,
    edit_buf: &mut String,
    uuid_fn: fn() -> Uuid,
    ble: &BleHandle,
    read_only: bool,
) {
    ui.horizontal(|ui| {
        ui.label(format!("{label:<24}"));
        let display = current.map_or("--".to_string(), |v| format!("{v}"));
        ui.monospace(format!("{display:<14}"));
        if !read_only {
            ui.add(egui::TextEdit::singleline(edit_buf).desired_width(100.0).vertical_align(egui::Align::Center));
        }
        if ui.button("Read").clicked() {
            ble.send(BleCommand::ReadCharacteristic(uuid_fn()));
        }
        if !read_only {
            if ui.button("Write").clicked() {
                if let Ok(val) = edit_buf.parse::<f32>() {
                    ble.send(BleCommand::WriteCharacteristic {
                        uuid: uuid_fn(),
                        data: codec::encode_f32_be(val),
                    });
                }
            }
        }
    });
}

pub fn register_row_u32(
    ui: &mut egui::Ui,
    label: &str,
    current: Option<u32>,
    edit_buf: &mut String,
    uuid_fn: fn() -> Uuid,
    ble: &BleHandle,
    read_only: bool,
) {
    ui.horizontal(|ui| {
        ui.label(format!("{label:<24}"));
        let display = current.map_or("--".to_string(), |v| format!("{v}"));
        ui.monospace(format!("{display:<14}"));
        if !read_only {
            ui.add(egui::TextEdit::singleline(edit_buf).desired_width(100.0).vertical_align(egui::Align::Center));
        }
        if ui.button("Read").clicked() {
            ble.send(BleCommand::ReadCharacteristic(uuid_fn()));
        }
        if !read_only {
            if ui.button("Write").clicked() {
                if let Ok(val) = edit_buf.parse::<u32>() {
                    ble.send(BleCommand::WriteCharacteristic {
                        uuid: uuid_fn(),
                        data: codec::encode_u32_be(val),
                    });
                }
            }
        }
    });
}

pub fn register_row_u8(
    ui: &mut egui::Ui,
    label: &str,
    current: Option<u8>,
    edit_buf: &mut String,
    uuid_fn: fn() -> Uuid,
    ble: &BleHandle,
    read_only: bool,
) {
    ui.horizontal(|ui| {
        ui.label(format!("{label:<24}"));
        let display = current.map_or("--".to_string(), |v| format!("{v}"));
        ui.monospace(format!("{display:<14}"));
        if !read_only {
            ui.add(egui::TextEdit::singleline(edit_buf).desired_width(100.0).vertical_align(egui::Align::Center));
        }
        if ui.button("Read").clicked() {
            ble.send(BleCommand::ReadCharacteristic(uuid_fn()));
        }
        if !read_only {
            if ui.button("Write").clicked() {
                if let Ok(val) = edit_buf.parse::<u8>() {
                    ble.send(BleCommand::WriteCharacteristic {
                        uuid: uuid_fn(),
                        data: codec::encode_u8(val),
                    });
                }
            }
        }
    });
}

pub fn register_row_string(
    ui: &mut egui::Ui,
    label: &str,
    current: Option<&str>,
    edit_buf: &mut String,
    uuid_fn: fn() -> Uuid,
    ble: &BleHandle,
    read_only: bool,
) {
    ui.horizontal(|ui| {
        ui.label(format!("{label:<24}"));
        let display = current.unwrap_or("--");
        ui.monospace(format!("{display:<14}"));
        if !read_only {
            ui.add(egui::TextEdit::singleline(edit_buf).desired_width(100.0).vertical_align(egui::Align::Center));
        }
        if ui.button("Read").clicked() {
            ble.send(BleCommand::ReadCharacteristic(uuid_fn()));
        }
        if !read_only {
            if ui.button("Write").clicked() {
                ble.send(BleCommand::WriteCharacteristic {
                    uuid: uuid_fn(),
                    data: codec::encode_string(edit_buf, 8),
                });
            }
        }
    });
}

/// A colored status indicator dot
pub fn status_indicator(ui: &mut egui::Ui, label: &str, active: bool) {
    let color = if active {
        COLOR_ERROR
    } else {
        COLOR_SUCCESS
    };
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 5.0, color);
        ui.label(label);
    });
}

// ── CSV Export ─────────────────────────────────────────────────────

/// Export history data (elapsed seconds + values) to a CSV file.
/// Reconstructs wall-clock timestamps from start_time.
pub fn export_history_csv(
    history: &VecDeque<(f64, f32)>,
    start_time: Option<std::time::Instant>,
    units_label: &str,
    default_filename: &str,
) {
    if history.is_empty() {
        return;
    }

    let file = rfd::FileDialog::new()
        .add_filter("CSV", &["csv"])
        .set_file_name(default_filename)
        .save_file();

    if let Some(path) = file {
        let mut wtr = match csv::Writer::from_path(&path) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to create CSV writer: {e}");
                return;
            }
        };

        let _ = wtr.write_record(["Elapsed (s)", "Timestamp", "Value", "Units"]);

        // Reconstruct wall-clock timestamps:
        // now - (total_elapsed - entry_elapsed) gives each entry's approximate time
        let now = chrono::Local::now();
        let total_elapsed = start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);

        for (elapsed, value) in history {
            let dt = now - chrono::Duration::milliseconds(
                ((total_elapsed - elapsed) * 1000.0) as i64,
            );
            let _ = wtr.write_record([
                format!("{elapsed:.3}"),
                dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
                format!("{value}"),
                units_label.to_string(),
            ]);
        }
        let _ = wtr.flush();
        log::info!("Exported {} points to {}", history.len(), path.display());
    }
}
