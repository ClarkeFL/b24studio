use eframe::egui;
use uuid::Uuid;
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::codec;

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
        egui::Color32::from_rgb(255, 80, 80)
    } else {
        egui::Color32::from_rgb(80, 200, 80)
    };
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 5.0, color);
        ui.label(label);
    });
}
