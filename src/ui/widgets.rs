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

/// Consistent page title: 22pt bold
pub fn page_header(ui: &mut egui::Ui, title: &str) {
    ui.label(egui::RichText::new(title).size(PAGE_HEADER_SIZE).strong());
}

/// Consistent section header: 17pt bold blue, with separator
pub fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(title)
            .size(SECTION_HEADER_SIZE)
            .strong()
            .color(SECTION_HEADER_COLOR),
    );
    ui.separator();
    ui.add_space(4.0);
}

/// Standalone help icon that reliably shows a tooltip on hover.
/// Renders as a small "(?)" badge with guaranteed minimum interaction area.
pub fn help_icon(ui: &mut egui::Ui, tooltip: &str) {
    let icon = egui::RichText::new("(?)")
        .size(13.0)
        .color(egui::Color32::from_rgb(120, 160, 210));
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
            ui.add(egui::TextEdit::singleline(edit_buf).desired_width(100.0));
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
            ui.add(egui::TextEdit::singleline(edit_buf).desired_width(100.0));
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
            ui.add(egui::TextEdit::singleline(edit_buf).desired_width(100.0));
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
            ui.add(egui::TextEdit::singleline(edit_buf).desired_width(100.0));
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
