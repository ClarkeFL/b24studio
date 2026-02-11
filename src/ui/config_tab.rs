use eframe::egui;
use uuid::Uuid;
use crate::state::AppState;
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::{uuids, codec};
use crate::protocol::types::DataUnits;

/// Fields that are safe to export/import (not device-specific)
#[derive(serde::Serialize, serde::Deserialize)]
struct ExportableConfig {
    data_rate: Option<u32>,
    resolution: Option<u8>,
    battery_threshold: Option<f32>,
    view_pin: Option<String>,
    system_zero: Option<f32>,
    config_pin: Option<u32>,
    cal_units: Option<u8>,
    data_tag: Option<String>,
}

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    ui.add_space(8.0);

    // Header with Refresh + Save + Export/Import
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Configuration").size(22.0).strong());
        ui.add_space(24.0);

        if ui.add_sized([120.0, 32.0], egui::Button::new(
            egui::RichText::new("Refresh All").size(15.0)
        )).clicked() {
            let cmd1 = BleCommand::ReadAll(uuids::all_config_uuids());
            state.track_send(&cmd1);
            ble.send(cmd1);
            let cmd2 = BleCommand::ReadCharacteristic(uuids::char_cal_units());
            state.track_send(&cmd2);
            ble.send(cmd2);
        }

        ui.add_space(8.0);

        if ui.add_sized([130.0, 32.0], egui::Button::new(
            egui::RichText::new("Save Changes").size(15.0)
        ).fill(egui::Color32::from_rgb(40, 120, 60))).clicked() {
            save_all_changes(state, ble);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_sized([100.0, 32.0], egui::Button::new(
                egui::RichText::new("Import").size(15.0)
            )).clicked() {
                import_config(state, ble);
            }
            ui.add_space(4.0);
            if ui.add_sized([100.0, 32.0], egui::Button::new(
                egui::RichText::new("Export").size(15.0)
            )).clicked() {
                export_config(state);
            }
        });
    });

    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);

        // ── Row 1: Device Info (left) | Communication (right) ─────────
        ui.columns(2, |cols| {
            // LEFT: Device Information (read-only)
            section_header(&mut cols[0], "Device Information");

            egui::Grid::new("cfg_device_info")
                .num_columns(2)
                .spacing([24.0, 10.0])
                .min_col_width(140.0)
                .show(&mut cols[0], |ui| {
                    field_label(ui, "Model");
                    field_value_or_spinner(ui,
                        state.config.model_name.as_deref().unwrap_or("--"),
                        uuids::char_model_name(), state);
                    ui.end_row();

                    field_label(ui, "Firmware Version");
                    field_value_or_spinner(ui,
                        &state.config.firmware_version
                            .map(|v| format!("{v:.2}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_firmware_ver(), state);
                    ui.end_row();

                    field_label(ui, "Serial Number");
                    field_value_or_spinner(ui,
                        &state.config.serial_number
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_serial_number(), state);
                    ui.end_row();

                    field_label(ui, "Battery");
                    let batt_str = state.config.battery_value
                        .map(|v| format!("{v:.3} V"))
                        .unwrap_or_else(|| "--".into());
                    let batt_uuid = uuids::char_battery_value();
                    if state.is_pending(&batt_uuid) {
                        ui.add(egui::Spinner::new().size(16.0));
                    } else {
                        let batt_color = state.config.battery_value
                            .map(|v| if v < 2.5 {
                                egui::Color32::from_rgb(255, 80, 80)
                            } else {
                                egui::Color32::from_rgb(80, 200, 80)
                            })
                            .unwrap_or(egui::Color32::GRAY);
                        ui.label(egui::RichText::new(&batt_str).size(16.0).monospace().color(batt_color));
                    }
                    ui.end_row();
                });

            // RIGHT: Communication
            section_header(&mut cols[1], "Communication");

            let input_width = 180.0;

            egui::Grid::new("cfg_comm")
                .num_columns(3)
                .spacing([16.0, 10.0])
                .min_col_width(80.0)
                .show(&mut cols[1], |ui| {
                    field_label(ui, "Data Rate (ms)");
                    field_value_or_spinner(ui,
                        &state.config.data_rate
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_data_rate(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.data_rate)
                            .hint_text("1000")
                            .font(egui::TextStyle::Body));
                    ui.end_row();

                    field_label(ui, "Resolution");
                    field_value_or_spinner(ui,
                        &state.config.resolution
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_resolution(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.resolution)
                            .hint_text("8 / 16 / 32 / 48 / 64")
                            .font(egui::TextStyle::Body));
                    ui.end_row();

                    field_label(ui, "Data Tag");
                    field_value_or_spinner(ui,
                        state.config.data_tag.as_deref().unwrap_or("--"),
                        uuids::char_data_tag(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.data_tag)
                            .hint_text("hex e.g. 7BE5")
                            .font(egui::TextStyle::Body));
                    ui.end_row();
                });
        });

        ui.add_space(16.0);

        // ── Row 2: Security (left) | Measurement (right) ─────────────
        ui.columns(2, |cols| {
            let input_width = 180.0;

            // LEFT: Security
            section_header(&mut cols[0], "Security");

            egui::Grid::new("cfg_security")
                .num_columns(3)
                .spacing([16.0, 10.0])
                .min_col_width(80.0)
                .show(&mut cols[0], |ui| {
                    field_label(ui, "Config PIN");
                    field_value_or_spinner(ui,
                        &state.config.config_pin
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_config_pin(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.config_pin)
                            .hint_text("0")
                            .font(egui::TextStyle::Body));
                    ui.end_row();

                    field_label(ui, "View PIN");
                    field_value_or_spinner(ui,
                        state.config.view_pin.as_deref().unwrap_or("--"),
                        uuids::char_view_pin(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.view_pin)
                            .hint_text("0000")
                            .font(egui::TextStyle::Body));
                    ui.end_row();
                });

            // RIGHT: Measurement
            section_header(&mut cols[1], "Measurement");

            egui::Grid::new("cfg_measurement")
                .num_columns(3)
                .spacing([16.0, 10.0])
                .min_col_width(80.0)
                .show(&mut cols[1], |ui| {
                    field_label(ui, "Battery Threshold");
                    field_value_or_spinner(ui,
                        &state.config.battery_threshold
                            .map(|v| format!("{v:.2} V"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_battery_thresh(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.battery_threshold)
                            .hint_text("2.4")
                            .font(egui::TextStyle::Body));
                    ui.end_row();

                    field_label(ui, "System Zero");
                    field_value_or_spinner(ui,
                        &state.config.system_zero
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_system_zero(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.system_zero)
                            .hint_text("0.0")
                            .font(egui::TextStyle::Body));
                    ui.end_row();
                });
        });

        ui.add_space(16.0);

        // ── Row 3: Units (full width) ─────────────────────────────────
        section_header(ui, "Units");

        let input_width = 180.0;

        egui::Grid::new("cfg_units")
            .num_columns(3)
            .spacing([16.0, 10.0])
            .min_col_width(80.0)
            .show(ui, |ui| {
                field_label(ui, "Calibration Units");
                let cu_str = state.calibration.cal_units
                    .map(|u| {
                        let du = DataUnits::from_byte(u);
                        format!("{} (0x{:02X})", du.label(), u)
                    })
                    .unwrap_or_else(|| "--".into());
                field_value_or_spinner(ui, &cu_str, uuids::char_cal_units(), state);

                // Dropdown for selecting calibration units
                let selected_label = if state.ui.edit.cal_units.is_empty() {
                    "Select unit...".to_string()
                } else if let Ok(byte_val) = state.ui.edit.cal_units.parse::<u8>() {
                    DataUnits::from_byte(byte_val).dropdown_label()
                } else {
                    "Select unit...".to_string()
                };
                egui::ComboBox::from_id_salt("cfg_cal_units_combo")
                    .selected_text(&selected_label)
                    .width(input_width)
                    .show_ui(ui, |ui| {
                        // Option to clear selection
                        if ui.selectable_label(state.ui.edit.cal_units.is_empty(), "-- None --").clicked() {
                            state.ui.edit.cal_units.clear();
                        }
                        for unit in DataUnits::ALL {
                            let label = unit.dropdown_label();
                            let byte_str = format!("{}", unit.to_byte());
                            let is_selected = state.ui.edit.cal_units == byte_str;
                            if ui.selectable_label(is_selected, &label).clicked() {
                                state.ui.edit.cal_units = byte_str;
                            }
                        }
                    });
                ui.end_row();
            });

        ui.add_space(24.0);
    });
}

// ── Save: write only fields the user has edited ────────────────────

fn send_tracked(state: &mut AppState, ble: &BleHandle, cmd: BleCommand) {
    state.track_send(&cmd);
    ble.send(cmd);
}

fn save_all_changes(state: &mut AppState, ble: &BleHandle) {
    if !state.ui.edit.data_rate.is_empty() {
        if let Ok(val) = state.ui.edit.data_rate.parse::<u32>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_data_rate(), data: codec::encode_u32_be(val),
            });
        }
    }
    if !state.ui.edit.resolution.is_empty() {
        if let Ok(val) = state.ui.edit.resolution.parse::<u8>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_resolution(), data: codec::encode_u8(val),
            });
        }
    }
    if !state.ui.edit.data_tag.is_empty() {
        if let Ok(val) = u16::from_str_radix(state.ui.edit.data_tag.trim(), 16) {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_data_tag(), data: codec::encode_u16_be(val),
            });
        }
    }
    if !state.ui.edit.config_pin.is_empty() {
        if let Ok(val) = state.ui.edit.config_pin.parse::<u32>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_config_pin(), data: codec::encode_u32_be(val),
            });
        }
    }
    if !state.ui.edit.view_pin.is_empty() {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_view_pin(),
            data: codec::encode_string(&state.ui.edit.view_pin, 5),
        });
    }
    if !state.ui.edit.battery_threshold.is_empty() {
        if let Ok(val) = state.ui.edit.battery_threshold.parse::<f32>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_battery_thresh(), data: codec::encode_f32_be(val),
            });
        }
    }
    if !state.ui.edit.system_zero.is_empty() {
        if let Ok(val) = state.ui.edit.system_zero.parse::<f32>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_system_zero(), data: codec::encode_f32_be(val),
            });
        }
    }
    if !state.ui.edit.cal_units.is_empty() {
        if let Ok(val) = state.ui.edit.cal_units.parse::<u8>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_cal_units(), data: codec::encode_u8(val),
            });
        }
    }

    // Re-read all to reflect changes
    send_tracked(state, ble, BleCommand::ReadAll(uuids::all_config_uuids()));
    send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_cal_units()));

    // Clear edit buffers
    state.ui.edit.data_rate.clear();
    state.ui.edit.resolution.clear();
    state.ui.edit.data_tag.clear();
    state.ui.edit.config_pin.clear();
    state.ui.edit.view_pin.clear();
    state.ui.edit.battery_threshold.clear();
    state.ui.edit.system_zero.clear();
    state.ui.edit.cal_units.clear();
}

// ── Export ──────────────────────────────────────────────────────────

fn export_config(state: &AppState) {
    let cfg = ExportableConfig {
        data_rate: state.config.data_rate,
        resolution: state.config.resolution,
        battery_threshold: state.config.battery_threshold,
        view_pin: state.config.view_pin.clone(),
        system_zero: state.config.system_zero,
        config_pin: state.config.config_pin,
        cal_units: state.calibration.cal_units,
        data_tag: state.config.data_tag.clone(),
    };

    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
        let task = rfd::FileDialog::new()
            .set_title("Export B24 Configuration")
            .add_filter("JSON", &["json"])
            .set_file_name("b24_config.json")
            .save_file();

        if let Some(path) = task {
            let _ = std::fs::write(path, json);
        }
    }
}

// ── Import ─────────────────────────────────────────────────────────

fn import_config(state: &mut AppState, ble: &BleHandle) {
    let task = rfd::FileDialog::new()
        .set_title("Import B24 Configuration")
        .add_filter("JSON", &["json"])
        .pick_file();

    if let Some(path) = task {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<ExportableConfig>(&data) {
                if let Some(val) = cfg.data_rate {
                    send_tracked(state, ble, BleCommand::WriteCharacteristic {
                        uuid: uuids::char_data_rate(), data: codec::encode_u32_be(val),
                    });
                }
                if let Some(val) = cfg.resolution {
                    send_tracked(state, ble, BleCommand::WriteCharacteristic {
                        uuid: uuids::char_resolution(), data: codec::encode_u8(val),
                    });
                }
                if let Some(val) = cfg.battery_threshold {
                    send_tracked(state, ble, BleCommand::WriteCharacteristic {
                        uuid: uuids::char_battery_thresh(), data: codec::encode_f32_be(val),
                    });
                }
                if let Some(ref pin) = cfg.view_pin {
                    send_tracked(state, ble, BleCommand::WriteCharacteristic {
                        uuid: uuids::char_view_pin(), data: codec::encode_string(pin, 5),
                    });
                }
                if let Some(val) = cfg.system_zero {
                    send_tracked(state, ble, BleCommand::WriteCharacteristic {
                        uuid: uuids::char_system_zero(), data: codec::encode_f32_be(val),
                    });
                }
                if let Some(val) = cfg.config_pin {
                    send_tracked(state, ble, BleCommand::WriteCharacteristic {
                        uuid: uuids::char_config_pin(), data: codec::encode_u32_be(val),
                    });
                }
                if let Some(val) = cfg.cal_units {
                    send_tracked(state, ble, BleCommand::WriteCharacteristic {
                        uuid: uuids::char_cal_units(), data: codec::encode_u8(val),
                    });
                }
                if let Some(ref tag) = cfg.data_tag {
                    if let Ok(val) = u16::from_str_radix(tag.trim(), 16) {
                        send_tracked(state, ble, BleCommand::WriteCharacteristic {
                            uuid: uuids::char_data_tag(), data: codec::encode_u16_be(val),
                        });
                    }
                }

                // Re-read all
                send_tracked(state, ble, BleCommand::ReadAll(uuids::all_config_uuids()));
                send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_cal_units()));
            }
        }
    }
}

// ── Helper functions ────────────────────────────────────────────

fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(title)
            .size(17.0)
            .strong()
            .color(egui::Color32::from_rgb(130, 170, 220)),
    );
    ui.separator();
    ui.add_space(4.0);
}

fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.add_sized([160.0, 26.0], egui::Label::new(
        egui::RichText::new(text).size(15.0)
    ));
}

fn field_value_or_spinner(ui: &mut egui::Ui, text: &str, uuid: Uuid, state: &AppState) {
    if state.is_pending(&uuid) {
        ui.add(egui::Spinner::new().size(16.0));
    } else {
        ui.label(egui::RichText::new(text).size(15.0).monospace());
    }
}
