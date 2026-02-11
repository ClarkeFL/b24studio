use eframe::egui;
use crate::state::AppState;
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::{uuids, codec};
use crate::protocol::types::DataUnits;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.heading("Configuration");
        ui.add_space(16.0);
        if ui.add_sized([140.0, 28.0], egui::Button::new("Read All")).clicked() {
            ble.send(BleCommand::ReadAll(uuids::all_config_uuids()));
        }
    });

    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        let grid_spacing = [20.0, 6.0];
        let label_width = 150.0;
        let value_width = 160.0;
        let edit_width = 120.0;

        // ── Device Info ─────────────────────────────────────────
        section_header(ui, "Device Information");

        egui::Grid::new("cfg_device_info")
            .num_columns(5)
            .spacing(grid_spacing)
            .min_col_width(40.0)
            .show(ui, |ui| {
                // Model (read-only)
                row_label(ui, "Model", label_width);
                row_value_str(ui, state.config.model_name.as_deref(), value_width);
                ui.label(""); // no edit
                row_read_btn(ui, uuids::char_model_name, ble);
                ui.label(""); // no write
                ui.end_row();

                // Firmware Version (read-only, f32)
                row_label(ui, "Firmware Version", label_width);
                let fw_str = state.config.firmware_version
                    .map(|v| format!("{v:.2}"))
                    .unwrap_or_else(|| "--".to_string());
                row_value(ui, &fw_str, value_width);
                ui.label("");
                row_read_btn(ui, uuids::char_firmware_ver, ble);
                ui.label("");
                ui.end_row();

                // Serial Number (read-only)
                row_label(ui, "Serial Number", label_width);
                row_value_str(ui, state.config.serial_number.as_deref(), value_width);
                ui.label("");
                row_read_btn(ui, uuids::char_serial_number, ble);
                ui.label("");
                ui.end_row();

                // Battery Value (read-only)
                row_label(ui, "Battery", label_width);
                let batt_str = state.config.battery_value
                    .map(|v| format!("{v:.3} V"))
                    .unwrap_or_else(|| "--".to_string());
                let batt_color = state.config.battery_value
                    .map(|v| if v < 2.5 {
                        egui::Color32::from_rgb(255, 80, 80)
                    } else {
                        egui::Color32::from_rgb(80, 200, 80)
                    })
                    .unwrap_or(egui::Color32::GRAY);
                ui.colored_label(batt_color, egui::RichText::new(&batt_str).monospace());
                ui.label("");
                row_read_btn(ui, uuids::char_battery_value, ble);
                ui.label("");
                ui.end_row();
            });

        ui.add_space(12.0);

        // ── Communication ───────────────────────────────────────
        section_header(ui, "Communication");

        egui::Grid::new("cfg_comm")
            .num_columns(5)
            .spacing(grid_spacing)
            .min_col_width(40.0)
            .show(ui, |ui| {
                // Data Rate
                row_label(ui, "Data Rate (ms)", label_width);
                row_value_u32(ui, state.config.data_rate, value_width);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.data_rate)
                    .desired_width(edit_width).hint_text("1000"));
                row_read_btn(ui, uuids::char_data_rate, ble);
                row_write_btn_u32(ui, &state.ui.edit.data_rate, uuids::char_data_rate, ble);
                ui.end_row();

                // Resolution
                row_label(ui, "Resolution", label_width);
                let res_str = state.config.resolution
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "--".to_string());
                row_value(ui, &res_str, value_width);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.resolution)
                    .desired_width(edit_width).hint_text("8/16/32/48/64"));
                row_read_btn(ui, uuids::char_resolution, ble);
                row_write_btn_u8(ui, &state.ui.edit.resolution, uuids::char_resolution, ble);
                ui.end_row();

                // Data Tag
                row_label(ui, "Data Tag", label_width);
                row_value_str(ui, state.config.data_tag.as_deref(), value_width);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.data_tag)
                    .desired_width(edit_width).hint_text("hex e.g. 7BE5"));
                row_read_btn(ui, uuids::char_data_tag, ble);
                if ui.button("Write").clicked() {
                    if let Ok(val) = u16::from_str_radix(state.ui.edit.data_tag.trim(), 16) {
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_data_tag(),
                            data: codec::encode_u16_be(val),
                        });
                    }
                }
                ui.end_row();
            });

        ui.add_space(12.0);

        // ── Security ────────────────────────────────────────────
        section_header(ui, "Security");

        egui::Grid::new("cfg_security")
            .num_columns(5)
            .spacing(grid_spacing)
            .min_col_width(40.0)
            .show(ui, |ui| {
                // Configuration PIN
                row_label(ui, "Config PIN", label_width);
                row_value_u32(ui, state.config.config_pin, value_width);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.config_pin)
                    .desired_width(edit_width).hint_text("9999"));
                row_read_btn(ui, uuids::char_config_pin, ble);
                row_write_btn_u32(ui, &state.ui.edit.config_pin, uuids::char_config_pin, ble);
                ui.end_row();

                // View PIN
                row_label(ui, "View PIN", label_width);
                row_value_u32(ui, state.config.view_pin, value_width);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.view_pin)
                    .desired_width(edit_width).hint_text("1234"));
                row_read_btn(ui, uuids::char_view_pin, ble);
                if ui.button("Write").clicked() {
                    if let Ok(val) = state.ui.edit.view_pin.parse::<u32>() {
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_view_pin(),
                            data: codec::encode_u32_be(val),
                        });
                    }
                }
                ui.end_row();
            });

        ui.add_space(12.0);

        // ── Measurement ─────────────────────────────────────────
        section_header(ui, "Measurement");

        egui::Grid::new("cfg_measurement")
            .num_columns(5)
            .spacing(grid_spacing)
            .min_col_width(40.0)
            .show(ui, |ui| {
                // Battery Threshold
                row_label(ui, "Battery Threshold", label_width);
                let bt_str = state.config.battery_threshold
                    .map(|v| format!("{v:.2} V"))
                    .unwrap_or_else(|| "--".to_string());
                row_value(ui, &bt_str, value_width);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.battery_threshold)
                    .desired_width(edit_width).hint_text("2.4"));
                row_read_btn(ui, uuids::char_battery_thresh, ble);
                row_write_btn_f32(ui, &state.ui.edit.battery_threshold, uuids::char_battery_thresh, ble);
                ui.end_row();

                // System Zero
                row_label(ui, "System Zero", label_width);
                let sz_str = state.config.system_zero
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "--".to_string());
                row_value(ui, &sz_str, value_width);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.system_zero)
                    .desired_width(edit_width).hint_text("0.0"));
                row_read_btn(ui, uuids::char_system_zero, ble);
                row_write_btn_f32(ui, &state.ui.edit.system_zero, uuids::char_system_zero, ble);
                ui.end_row();
            });

        ui.add_space(12.0);

        // ── Calibration Units ───────────────────────────────────
        section_header(ui, "Units");

        egui::Grid::new("cfg_units")
            .num_columns(5)
            .spacing(grid_spacing)
            .min_col_width(40.0)
            .show(ui, |ui| {
                // Cal Units
                row_label(ui, "Calibration Units", label_width);
                let cu_str = state.calibration.cal_units
                    .map(|u| {
                        let du = DataUnits::from_byte(u);
                        format!("{} ({})", du.label(), u)
                    })
                    .unwrap_or_else(|| "--".to_string());
                row_value(ui, &cu_str, value_width);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.cal_units)
                    .desired_width(edit_width).hint_text("unit byte"));
                row_read_btn(ui, uuids::char_cal_units, ble);
                row_write_btn_u8(ui, &state.ui.edit.cal_units, uuids::char_cal_units, ble);
                ui.end_row();
            });
    });
}

// ── Helper functions ────────────────────────────────────────────

fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(title)
            .size(14.0)
            .strong()
            .color(egui::Color32::from_rgb(130, 170, 220)),
    );
    ui.separator();
    ui.add_space(2.0);
}

fn row_label(ui: &mut egui::Ui, text: &str, width: f32) {
    ui.add_sized([width, 20.0], egui::Label::new(text));
}

fn row_value(ui: &mut egui::Ui, text: &str, _width: f32) {
    ui.monospace(text);
}

fn row_value_str(ui: &mut egui::Ui, val: Option<&str>, _width: f32) {
    ui.monospace(val.unwrap_or("--"));
}

fn row_value_u32(ui: &mut egui::Ui, val: Option<u32>, _width: f32) {
    let s = val.map(|v| format!("{v}")).unwrap_or_else(|| "--".to_string());
    ui.monospace(&s);
}

fn row_read_btn(ui: &mut egui::Ui, uuid_fn: fn() -> uuid::Uuid, ble: &BleHandle) {
    if ui.button("Read").clicked() {
        ble.send(BleCommand::ReadCharacteristic(uuid_fn()));
    }
}

fn row_write_btn_f32(ui: &mut egui::Ui, edit_buf: &str, uuid_fn: fn() -> uuid::Uuid, ble: &BleHandle) {
    if ui.button("Write").clicked() {
        if let Ok(val) = edit_buf.parse::<f32>() {
            ble.send(BleCommand::WriteCharacteristic {
                uuid: uuid_fn(),
                data: codec::encode_f32_be(val),
            });
        }
    }
}

fn row_write_btn_u32(ui: &mut egui::Ui, edit_buf: &str, uuid_fn: fn() -> uuid::Uuid, ble: &BleHandle) {
    if ui.button("Write").clicked() {
        if let Ok(val) = edit_buf.parse::<u32>() {
            ble.send(BleCommand::WriteCharacteristic {
                uuid: uuid_fn(),
                data: codec::encode_u32_be(val),
            });
        }
    }
}

fn row_write_btn_u8(ui: &mut egui::Ui, edit_buf: &str, uuid_fn: fn() -> uuid::Uuid, ble: &BleHandle) {
    if ui.button("Write").clicked() {
        if let Ok(val) = edit_buf.parse::<u8>() {
            ble.send(BleCommand::WriteCharacteristic {
                uuid: uuid_fn(),
                data: codec::encode_u8(val),
            });
        }
    }
}
