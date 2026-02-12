use eframe::egui;
use uuid::Uuid;
use crate::state::AppState;
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::{uuids, codec};
use crate::protocol::types::{DataUnits, DeviceAction, SensitivityRange, AdvancedParam, ParamType};
use crate::ui::widgets;

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
    sensitivity_range: Option<u8>,
    data_gain: Option<f32>,
    data_offset: Option<f32>,
    cal_pin: Option<u32>,
    #[serde(default)]
    local_name: Option<String>,
    #[serde(default)]
    data_units: Option<u8>,
}

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    ui.add_space(8.0);

    // Header with Refresh + Save + Export/Import
    ui.horizontal(|ui| {
        widgets::page_header(ui, "Configuration");
        ui.add_space(24.0);

        if ui.add_sized([120.0, 32.0], egui::Button::new(
            egui::RichText::new("Refresh All").size(15.0)
        )).clicked() {
            send_tracked(state, ble, BleCommand::ReadAll(uuids::all_config_uuids()));
            send_tracked(state, ble, BleCommand::ReadAll(uuids::all_calibration_uuids()));
            send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_data_units()));
            send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_gap_device_name()));
        }

        ui.add_space(8.0);

        if ui.add_sized([130.0, 32.0], egui::Button::new(
            egui::RichText::new("Save Changes").size(15.0).color(egui::Color32::WHITE)
        ).fill(widgets::COLOR_BTN_GREEN)).clicked() {
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
            widgets::section_header(&mut cols[0], "Device Information");

            let info_input_width = 180.0;

            egui::Grid::new("cfg_device_info")
                .num_columns(3)
                .spacing([16.0, 10.0])
                .min_col_width(80.0)
                .show(&mut cols[0], |ui| {
                    field_label(ui, "Model");
                    field_value_or_spinner(ui,
                        state.config.model_name.as_deref().unwrap_or("--"),
                        uuids::char_model_name(), state);
                    ui.label(""); // empty column
                    ui.end_row();

                    field_label(ui, "Firmware Version");
                    field_value_or_spinner(ui,
                        &state.config.firmware_version
                            .map(|v| format!("{v:.2}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_firmware_ver(), state);
                    ui.label("");
                    ui.end_row();

                    field_label(ui, "Serial Number");
                    field_value_or_spinner(ui,
                        &state.config.serial_number
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_serial_number(), state);
                    ui.label("");
                    ui.end_row();

                    field_label(ui, "Battery");
                    let batt_str = state.config.battery_value
                        .map(|v| format!("{v:.3} V"))
                        .unwrap_or_else(|| "--".into());
                    let batt_uuid = uuids::char_battery_value();
                    if state.is_pending(&batt_uuid) {
                        ui.add(egui::Spinner::new().size(widgets::SPINNER_SIZE));
                    } else {
                        let batt_color = state.config.battery_value
                            .map(|v| if v < 2.5 {
                                widgets::COLOR_ERROR
                            } else {
                                widgets::COLOR_SUCCESS
                            })
                            .unwrap_or(widgets::muted_text(ui.visuals().dark_mode));
                        ui.label(egui::RichText::new(&batt_str).size(16.0).monospace().color(batt_color));
                    }
                    ui.label("");
                    ui.end_row();

                    field_label_help(ui, "Local Name", Some(
                        "The BLE broadcast name of this device.\n\
                         This is the name that appears during scanning.\n\
                         Change it to identify your device (e.g. \"B24-Tank1\")."
                    ));
                    field_value_or_spinner(ui,
                        state.config.local_name.as_deref().unwrap_or("--"),
                        uuids::char_gap_device_name(), state);
                    ui.add_sized([info_input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.local_name)
                            .hint_text("e.g. B24")
                            .vertical_align(egui::Align::Center)
                            .font(egui::TextStyle::Body));
                    ui.end_row();
                });

            // RIGHT: Communication
            widgets::section_header(&mut cols[1], "Communication");

            let input_width = 180.0;

            egui::Grid::new("cfg_comm")
                .num_columns(3)
                .spacing([16.0, 10.0])
                .min_col_width(80.0)
                .show(&mut cols[1], |ui| {
                    field_label_help(ui, "Data Rate (ms)", Some(
                        "Interval between measurements in milliseconds.\n\
                         Lower values = faster updates but more power consumption.\n\
                         Typical: 1000 ms (1 reading/second)."
                    ));
                    field_value_or_spinner(ui,
                        &state.config.data_rate
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_data_rate(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.data_rate)
                            .hint_text("1000")
                            .vertical_align(egui::Align::Center)
                            .font(egui::TextStyle::Body));
                    ui.end_row();

                    field_label_help(ui, "Resolution", Some(
                        "ADC resolution in bits. Higher = more precise but slower.\n\
                         Valid values: 8, 16, 32, 48, 64.\n\
                         Typical: 16 for fast readings, 64 for precision."
                    ));
                    field_value_or_spinner(ui,
                        &state.config.resolution
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_resolution(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.resolution)
                            .hint_text("8 / 16 / 32 / 48 / 64")
                            .vertical_align(egui::Align::Center)
                            .font(egui::TextStyle::Body));
                    ui.end_row();

                    field_label_help(ui, "Data Tag", Some(
                        "4-digit hex identifier (e.g. 7BE5) for grouping devices.\n\
                         Used to identify this transmitter in advertising broadcasts.\n\
                         Enter as hexadecimal (0000–FFFF)."
                    ));
                    field_value_or_spinner(ui,
                        state.config.data_tag.as_deref().unwrap_or("--"),
                        uuids::char_data_tag(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.data_tag)
                            .hint_text("hex e.g. 7BE5")
                            .vertical_align(egui::Align::Center)
                            .font(egui::TextStyle::Body));
                    ui.end_row();

                    field_label_help(ui, "Data Units", Some(
                        "The measurement units sent in BLE advertising packets.\n\
                         This determines the unit label shown alongside the data value\n\
                         in scan results and View mode."
                    ));
                    let du_str = state.config.data_units
                        .map(|u| {
                            let du = DataUnits::from_byte(u);
                            format!("{} ({})", du.label(), u)
                        })
                        .unwrap_or_else(|| "--".to_string());
                    field_value_or_spinner(ui, &du_str, uuids::char_data_units(), state);

                    let du_selected_label = if state.ui.edit.data_units_display.is_empty() {
                        "Select unit...".to_string()
                    } else if let Ok(byte_val) = state.ui.edit.data_units_display.parse::<u8>() {
                        DataUnits::from_byte(byte_val).dropdown_label()
                    } else {
                        "Select unit...".to_string()
                    };
                    egui::ComboBox::from_id_salt("cfg_data_units_combo")
                        .selected_text(&du_selected_label)
                        .width(input_width)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(state.ui.edit.data_units_display.is_empty(), "-- None --").clicked() {
                                state.ui.edit.data_units_display.clear();
                            }
                            for unit in DataUnits::ALL {
                                let label = unit.dropdown_label();
                                let byte_str = format!("{}", unit.to_byte());
                                let is_selected = state.ui.edit.data_units_display == byte_str;
                                if ui.selectable_label(is_selected, &label).clicked() {
                                    state.ui.edit.data_units_display = byte_str;
                                }
                            }
                        });
                    ui.end_row();
                });
        });

        ui.add_space(16.0);

        // ── Row 2: Security (left) | Measurement (right) ─────────────
        ui.columns(2, |cols| {
            let input_width = 180.0;

            // LEFT: Security
            widgets::section_header(&mut cols[0], "Security");

            egui::Grid::new("cfg_security")
                .num_columns(3)
                .spacing([16.0, 10.0])
                .min_col_width(80.0)
                .show(&mut cols[0], |ui| {
                    field_label_help(ui, "Config PIN", Some(
                        "Configuration PIN required to access device settings.\n\
                         Must be entered within 5 seconds of connecting.\n\
                         Default: 0. Change this to protect your device."
                    ));
                    field_value_or_spinner(ui,
                        &state.config.config_pin
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_config_pin(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.config_pin)
                            .hint_text("0")
                            .vertical_align(egui::Align::Center)
                            .font(egui::TextStyle::Body));
                    ui.end_row();

                    field_label_help(ui, "View PIN", Some(
                        "4-digit PIN used to decode advertising broadcast data.\n\
                         Allows viewing live values without connecting.\n\
                         Enter in View mode on the Connect tab. Default: 0000."
                    ));
                    field_value_or_spinner(ui,
                        state.config.view_pin.as_deref().unwrap_or("--"),
                        uuids::char_view_pin(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.view_pin)
                            .hint_text("0000")
                            .vertical_align(egui::Align::Center)
                            .font(egui::TextStyle::Body));
                    ui.end_row();
                });

            // RIGHT: Measurement
            widgets::section_header(&mut cols[1], "Measurement");

            egui::Grid::new("cfg_measurement")
                .num_columns(3)
                .spacing([16.0, 10.0])
                .min_col_width(80.0)
                .show(&mut cols[1], |ui| {
                    field_label_help(ui, "Battery Threshold", Some(
                        "Voltage level below which the battery low alert triggers.\n\
                         Typical: 2.4V. The status bar shows battery voltage in real-time."
                    ));
                    field_value_or_spinner(ui,
                        &state.config.battery_threshold
                            .map(|v| format!("{v:.2} V"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_battery_thresh(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.battery_threshold)
                            .hint_text("2.4")
                            .vertical_align(egui::Align::Center)
                            .font(egui::TextStyle::Body));
                    ui.end_row();

                    field_label_help(ui, "System Zero", Some(
                        "Zero offset applied to the measurement system.\n\
                         Set to 0.0 unless you need to offset the zero point.\n\
                         Use Tare (Device Actions) for temporary zeroing instead."
                    ));
                    field_value_or_spinner(ui,
                        &state.config.system_zero
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".into()),
                        uuids::char_system_zero(), state);
                    ui.add_sized([input_width, 30.0],
                        egui::TextEdit::singleline(&mut state.ui.edit.system_zero)
                            .hint_text("0.0")
                            .vertical_align(egui::Align::Center)
                            .font(egui::TextStyle::Body));
                    ui.end_row();
                });
        });

        ui.add_space(16.0);

        // ── Row 3: Calibration Registers (two-column) ─────────────────
        ui.columns(2, |cols| {
            let edit_w = 150.0;
            let label_w = 160.0;

            // LEFT column
            widgets::section_header(&mut cols[0], "Calibration Registers");

            egui::Grid::new("cfg_cal_left")
                .num_columns(3)
                .spacing([12.0, 8.0])
                .min_col_width(40.0)
                .show(&mut cols[0], |ui| {
                    // Sensitivity Range
                    ui.horizontal(|ui| {
                        ui.add_sized([label_w - 20.0, 26.0], egui::Label::new("Sensitivity Range"));
                        widgets::help_icon(ui, "Input sensitivity range for the strain gauge bridge.\n0 = ±2.5 mV/V, 1 = ±5 mV/V, 2 = ±10 mV/V, 3 = ±20 mV/V.\nChoose the smallest range that covers your sensor output.");
                    });
                    let sr_str = state.calibration.sensitivity_range
                        .and_then(SensitivityRange::from_byte)
                        .map(|s| s.label().to_string())
                        .unwrap_or_else(|| state.calibration.sensitivity_range
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".to_string()));
                    field_value_or_spinner(ui, &sr_str, uuids::char_sens_range(), state);
                    ui.add_sized([edit_w, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.sensitivity_range)
                        .hint_text("0-3")
                        .vertical_align(egui::Align::Center));
                    ui.end_row();

                    // Data Gain (unit conversion)
                    ui.horizontal(|ui| {
                        ui.add_sized([label_w - 20.0, 26.0], egui::Label::new("Data Gain"));
                        widgets::help_icon(ui, "Multiplier applied after linearisation for unit conversion.\nFormula: output = (linearised_value × Data Gain) + Data Offset.\nDefault: 1.0. Change only if converting between unit systems.");
                    });
                    let dg = state.calibration.data_gain
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    field_value_or_spinner(ui, &dg, uuids::char_data_gain(), state);
                    ui.add_sized([edit_w, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.data_gain)
                        .hint_text("1.0")
                        .vertical_align(egui::Align::Center));
                    ui.end_row();

                    // Data Offset (unit conversion)
                    ui.horizontal(|ui| {
                        ui.add_sized([label_w - 20.0, 26.0], egui::Label::new("Data Offset"));
                        widgets::help_icon(ui, "Offset applied after linearisation for unit conversion.\nFormula: output = (linearised_value × Data Gain) + Data Offset.\nDefault: 0.0. Change only if converting between unit systems.");
                    });
                    let do_ = state.calibration.data_offset
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    field_value_or_spinner(ui, &do_, uuids::char_data_offset(), state);
                    ui.add_sized([edit_w, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.data_offset)
                        .hint_text("0.0")
                        .vertical_align(egui::Align::Center));
                    ui.end_row();

                    // Cal PIN
                    ui.horizontal(|ui| {
                        ui.add_sized([label_w - 20.0, 26.0], egui::Label::new("Calibration PIN"));
                        widgets::help_icon(ui, "PIN required to write calibration data to the device.\nMust match the device's stored calibration PIN.\nDefault: 0.");
                    });
                    let cp = state.calibration.cal_pin
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    field_value_or_spinner(ui, &cp, uuids::char_cal_pin(), state);
                    ui.add_sized([edit_w, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.cal_pin)
                        .vertical_align(egui::Align::Center));
                    ui.end_row();
                });

            // RIGHT column
            cols[1].add_space(30.0); // align with left header

            egui::Grid::new("cfg_cal_right")
                .num_columns(3)
                .spacing([12.0, 8.0])
                .min_col_width(40.0)
                .show(&mut cols[1], |ui| {
                    // Cal Units (dropdown)
                    ui.horizontal(|ui| {
                        ui.add_sized([label_w - 20.0, 26.0], egui::Label::new("Calibration Units"));
                        widgets::help_icon(ui, "Engineering units for calibrated output values.\nSelect the unit that matches your measurement (e.g. N m, kg, lbf).\nThis label is shown alongside the live value.");
                    });
                    let cu = state.calibration.cal_units
                        .map(|u| {
                            let du = DataUnits::from_byte(u);
                            format!("{} ({})", du.label(), u)
                        })
                        .unwrap_or_else(|| "--".to_string());
                    field_value_or_spinner(ui, &cu, uuids::char_cal_units(), state);

                    let selected_label = if state.ui.edit.cal_units_display.is_empty() {
                        "Select unit...".to_string()
                    } else if let Ok(byte_val) = state.ui.edit.cal_units_display.parse::<u8>() {
                        DataUnits::from_byte(byte_val).dropdown_label()
                    } else {
                        "Select unit...".to_string()
                    };
                    egui::ComboBox::from_id_salt("cfg_cal_units_combo")
                        .selected_text(&selected_label)
                        .width(edit_w)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(state.ui.edit.cal_units_display.is_empty(), "-- None --").clicked() {
                                state.ui.edit.cal_units_display.clear();
                            }
                            for unit in DataUnits::ALL {
                                let label = unit.dropdown_label();
                                let byte_str = format!("{}", unit.to_byte());
                                let is_selected = state.ui.edit.cal_units_display == byte_str;
                                if ui.selectable_label(is_selected, &label).clicked() {
                                    state.ui.edit.cal_units_display = byte_str;
                                }
                            }
                        });
                    ui.end_row();

                    // Coefficient (read-only)
                    ui.add_sized([label_w, 26.0], egui::Label::new("Coefficient (@Idx)"));
                    let cf = state.calibration.coefficient
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    field_value_or_spinner(ui, &cf, uuids::char_coeff_at_idx(), state);
                    ui.label("");
                    ui.end_row();

                    // Base Value (read-only)
                    ui.add_sized([label_w, 26.0], egui::Label::new("Base Value"));
                    let bv = state.calibration.base_value
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    field_value_or_spinner(ui, &bv, uuids::char_base_value(), state);
                    ui.label("");
                    ui.end_row();

                    // Base Units (read-only)
                    ui.add_sized([label_w, 26.0], egui::Label::new("Base Units"));
                    let bu = state.calibration.base_units
                        .map(|u| {
                            let du = DataUnits::from_byte(u);
                            format!("{} ({})", du.label(), u)
                        })
                        .unwrap_or_else(|| "--".to_string());
                    field_value_or_spinner(ui, &bu, uuids::char_base_units(), state);
                    ui.label("");
                    ui.end_row();
                });
        });

        ui.add_space(16.0);

        // ── Row 4: Device Actions (full width) ────────────────────────
        widgets::section_header(ui, "Device Actions");

        ui.horizontal_wrapped(|ui| {
            let actions = [
                (DeviceAction::Tare, egui::Color32::from_rgb(50, 100, 180)),
                (DeviceAction::ResetTare, egui::Color32::from_rgb(80, 80, 100)),
                (DeviceAction::ShuntCalOn, egui::Color32::from_rgb(50, 130, 80)),
                (DeviceAction::ShuntCalOff, egui::Color32::from_rgb(80, 80, 100)),
                (DeviceAction::ResetPeakTrough, egui::Color32::from_rgb(180, 130, 50)),
                (DeviceAction::CalculateCoefficients, egui::Color32::from_rgb(50, 130, 130)),
                (DeviceAction::Reboot, egui::Color32::from_rgb(180, 100, 50)),
                (DeviceAction::RestoreEepromDefaults, widgets::COLOR_BTN_RED),
            ];
            for (action, color) in actions {
                if ui.add_sized(
                    [170.0, 32.0],
                    egui::Button::new(egui::RichText::new(action.label()).size(14.0).color(egui::Color32::WHITE)).fill(color),
                ).clicked() {
                    ble.send(BleCommand::ExecuteAction(action));
                }
            }
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
    // Config fields
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
    if !state.ui.edit.local_name.is_empty() {
        let name = state.ui.edit.local_name.trim();
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_gap_device_name(),
            data: name.as_bytes().to_vec(),
        });
    }
    if !state.ui.edit.data_units_display.is_empty() {
        if let Ok(v) = state.ui.edit.data_units_display.parse::<u8>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_data_units(), data: codec::encode_u8(v),
            });
        }
    }

    // Calibration register fields
    if !state.ui.edit.sensitivity_range.is_empty() {
        if let Ok(v) = state.ui.edit.sensitivity_range.parse::<u8>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_sens_range(), data: codec::encode_u8(v),
            });
        }
    }
    if !state.ui.edit.data_gain.is_empty() {
        if let Ok(v) = state.ui.edit.data_gain.parse::<f32>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_data_gain(), data: codec::encode_f32_be(v),
            });
        }
    }
    if !state.ui.edit.data_offset.is_empty() {
        if let Ok(v) = state.ui.edit.data_offset.parse::<f32>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_data_offset(), data: codec::encode_f32_be(v),
            });
        }
    }
    if !state.ui.edit.cal_pin.is_empty() {
        if let Ok(v) = state.ui.edit.cal_pin.parse::<u32>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_cal_pin(), data: codec::encode_u32_be(v),
            });
        }
    }
    if !state.ui.edit.cal_units_display.is_empty() {
        if let Ok(v) = state.ui.edit.cal_units_display.parse::<u8>() {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_cal_units(), data: codec::encode_u8(v),
            });
        }
    }

    // Re-read all to reflect changes
    send_tracked(state, ble, BleCommand::ReadAll(uuids::all_config_uuids()));
    send_tracked(state, ble, BleCommand::ReadAll(uuids::all_calibration_uuids()));
    send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_data_units()));
    send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_gap_device_name()));

    // Clear edit buffers
    state.ui.edit.data_rate.clear();
    state.ui.edit.resolution.clear();
    state.ui.edit.data_tag.clear();
    state.ui.edit.config_pin.clear();
    state.ui.edit.view_pin.clear();
    state.ui.edit.battery_threshold.clear();
    state.ui.edit.system_zero.clear();
    state.ui.edit.local_name.clear();
    state.ui.edit.data_units_display.clear();
    state.ui.edit.cal_units.clear();
    state.ui.edit.sensitivity_range.clear();
    state.ui.edit.data_gain.clear();
    state.ui.edit.data_offset.clear();
    state.ui.edit.cal_pin.clear();
    state.ui.edit.cal_units_display.clear();
}

// ── Export ──────────────────────────────────────────────────────────

fn export_config(state: &AppState) {
    let mut lines: Vec<String> = Vec::new();

    lines.push("# B24 Configuration Export".to_string());
    lines.push(format!("# Exported: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    if let Some(ref model) = state.config.model_name {
        lines.push(format!("# Model: {}", model));
    }
    if let Some(sn) = state.config.serial_number {
        lines.push(format!("# Serial: {}", sn));
    }
    if let Some(fw) = state.config.firmware_version {
        lines.push(format!("# Firmware: {:.2}", fw));
    }
    lines.push(String::new());

    lines.push("[Configuration]".to_string());
    write_opt_u32(&mut lines, "data_rate", state.config.data_rate);
    write_opt_u8(&mut lines, "resolution", state.config.resolution);
    write_opt_f32(&mut lines, "battery_threshold", state.config.battery_threshold);
    write_opt_str(&mut lines, "view_pin", state.config.view_pin.as_deref());
    write_opt_f32(&mut lines, "system_zero", state.config.system_zero);
    write_opt_u32(&mut lines, "config_pin", state.config.config_pin);
    write_opt_str(&mut lines, "data_tag", state.config.data_tag.as_deref());
    write_opt_str(&mut lines, "local_name", state.config.local_name.as_deref());
    write_opt_u8(&mut lines, "data_units", state.config.data_units);
    lines.push(String::new());

    lines.push("[Calibration]".to_string());
    write_opt_u8(&mut lines, "sensitivity_range", state.calibration.sensitivity_range);
    write_opt_f32(&mut lines, "data_gain", state.calibration.data_gain);
    write_opt_f32(&mut lines, "data_offset", state.calibration.data_offset);
    write_opt_u32(&mut lines, "cal_pin", state.calibration.cal_pin);
    write_opt_u8(&mut lines, "cal_units", state.calibration.cal_units);
    lines.push(String::new());

    lines.push("[Advanced]".to_string());
    for param in AdvancedParam::ALL {
        let index = param.index();
        let is_readonly = *param == AdvancedParam::PeakValue || *param == AdvancedParam::TroughValue;
        if is_readonly { continue; }
        let val_str = match param.data_type() {
            ParamType::Float => state.advanced.get_f32(index).map(|v| format!("{v}")),
            ParamType::Uint32 => state.advanced.get_u32(index).map(|v| format!("{v}")),
            ParamType::Uint8 => state.advanced.get_u8(index).map(|v| format!("{v}")),
        };
        if let Some(val) = val_str {
            lines.push(format!("{} = {}", param.label(), val));
        }
    }
    lines.push(String::new());

    let content = lines.join("\n");

    let tag = state.config.data_tag.as_deref().unwrap_or("0000");
    let filename = format!("{}_configuration_export.txt", tag);

    let task = rfd::FileDialog::new()
        .set_title("Export B24 Configuration")
        .add_filter("Text files", &["txt"])
        .set_file_name(&filename)
        .save_file();

    if let Some(path) = task {
        let _ = std::fs::write(path, content);
    }
}

fn write_opt_u32(lines: &mut Vec<String>, key: &str, val: Option<u32>) {
    if let Some(v) = val { lines.push(format!("{} = {}", key, v)); }
}

fn write_opt_u8(lines: &mut Vec<String>, key: &str, val: Option<u8>) {
    if let Some(v) = val { lines.push(format!("{} = {}", key, v)); }
}

fn write_opt_f32(lines: &mut Vec<String>, key: &str, val: Option<f32>) {
    if let Some(v) = val { lines.push(format!("{} = {}", key, v)); }
}

fn write_opt_str(lines: &mut Vec<String>, key: &str, val: Option<&str>) {
    if let Some(v) = val { lines.push(format!("{} = {}", key, v)); }
}

// ── Import ─────────────────────────────────────────────────────────

fn import_config(state: &mut AppState, ble: &BleHandle) {
    let task = rfd::FileDialog::new()
        .set_title("Import B24 Configuration")
        .add_filter("Text files", &["txt", "json"])
        .pick_file();

    let Some(path) = task else { return; };
    let Ok(data) = std::fs::read_to_string(&path) else { return; };

    // Try JSON format first (backward compatibility)
    if let Ok(cfg) = serde_json::from_str::<ExportableConfig>(&data) {
        import_from_exportable(state, ble, &cfg);
        return;
    }

    // Parse key=value text format
    let mut kvs: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') { continue; }
        if let Some((key, val)) = line.split_once('=') {
            kvs.insert(key.trim().to_lowercase().replace(' ', "_"), val.trim().to_string());
        }
    }

    // Configuration section
    if let Some(v) = kvs.get("data_rate").and_then(|s| s.parse::<u32>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_data_rate(), data: codec::encode_u32_be(v),
        });
    }
    if let Some(v) = kvs.get("resolution").and_then(|s| s.parse::<u8>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_resolution(), data: codec::encode_u8(v),
        });
    }
    if let Some(v) = kvs.get("battery_threshold").and_then(|s| s.parse::<f32>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_battery_thresh(), data: codec::encode_f32_be(v),
        });
    }
    if let Some(v) = kvs.get("view_pin") {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_view_pin(), data: codec::encode_string(v, 5),
        });
    }
    if let Some(v) = kvs.get("system_zero").and_then(|s| s.parse::<f32>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_system_zero(), data: codec::encode_f32_be(v),
        });
    }
    if let Some(v) = kvs.get("config_pin").and_then(|s| s.parse::<u32>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_config_pin(), data: codec::encode_u32_be(v),
        });
    }
    if let Some(v) = kvs.get("data_tag") {
        if let Ok(val) = u16::from_str_radix(v.trim(), 16) {
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_data_tag(), data: codec::encode_u16_be(val),
            });
        }
    }
    if let Some(v) = kvs.get("local_name") {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_gap_device_name(),
            data: v.as_bytes().to_vec(),
        });
    }
    if let Some(v) = kvs.get("data_units").and_then(|s| s.parse::<u8>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_data_units(), data: codec::encode_u8(v),
        });
    }

    // Calibration section
    if let Some(v) = kvs.get("sensitivity_range").and_then(|s| s.parse::<u8>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_sens_range(), data: codec::encode_u8(v),
        });
    }
    if let Some(v) = kvs.get("data_gain").and_then(|s| s.parse::<f32>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_data_gain(), data: codec::encode_f32_be(v),
        });
    }
    if let Some(v) = kvs.get("data_offset").and_then(|s| s.parse::<f32>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_data_offset(), data: codec::encode_f32_be(v),
        });
    }
    if let Some(v) = kvs.get("cal_pin").and_then(|s| s.parse::<u32>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_cal_pin(), data: codec::encode_u32_be(v),
        });
    }
    if let Some(v) = kvs.get("cal_units").and_then(|s| s.parse::<u8>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_cal_units(), data: codec::encode_u8(v),
        });
    }

    // Advanced section - match by label (lowercased, spaces->underscores)
    for param in AdvancedParam::ALL {
        let is_readonly = *param == AdvancedParam::PeakValue || *param == AdvancedParam::TroughValue;
        if is_readonly { continue; }
        let key = param.label().to_lowercase().replace(' ', "_");
        if let Some(val_str) = kvs.get(&key) {
            let index = param.index();
            match param.data_type() {
                ParamType::Float => {
                    if let Ok(v) = val_str.parse::<f32>() {
                        send_tracked(state, ble, BleCommand::WriteAdvanced {
                            index, data: v.to_be_bytes().to_vec(),
                        });
                    }
                }
                ParamType::Uint32 => {
                    if let Ok(v) = val_str.parse::<u32>() {
                        send_tracked(state, ble, BleCommand::WriteAdvanced {
                            index, data: v.to_be_bytes().to_vec(),
                        });
                    }
                }
                ParamType::Uint8 => {
                    if let Ok(v) = val_str.parse::<u8>() {
                        send_tracked(state, ble, BleCommand::WriteAdvanced {
                            index, data: vec![v],
                        });
                    }
                }
            }
        }
    }

    // Re-read all to reflect changes
    send_tracked(state, ble, BleCommand::ReadAll(uuids::all_config_uuids()));
    send_tracked(state, ble, BleCommand::ReadAll(uuids::all_calibration_uuids()));
    for param in AdvancedParam::ALL {
        send_tracked(state, ble, BleCommand::ReadAdvanced { index: param.index() });
    }
}

fn import_from_exportable(state: &mut AppState, ble: &BleHandle, cfg: &ExportableConfig) {
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
    if let Some(val) = cfg.sensitivity_range {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_sens_range(), data: codec::encode_u8(val),
        });
    }
    if let Some(val) = cfg.data_gain {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_data_gain(), data: codec::encode_f32_be(val),
        });
    }
    if let Some(val) = cfg.data_offset {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_data_offset(), data: codec::encode_f32_be(val),
        });
    }
    if let Some(val) = cfg.cal_pin {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_cal_pin(), data: codec::encode_u32_be(val),
        });
    }
    if let Some(ref name) = cfg.local_name {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_gap_device_name(),
            data: name.as_bytes().to_vec(),
        });
    }
    if let Some(val) = cfg.data_units {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_data_units(), data: codec::encode_u8(val),
        });
    }
    send_tracked(state, ble, BleCommand::ReadAll(uuids::all_config_uuids()));
    send_tracked(state, ble, BleCommand::ReadAll(uuids::all_calibration_uuids()));
}

// ── Helper functions ────────────────────────────────────────────

fn field_label(ui: &mut egui::Ui, text: &str) {
    field_label_help(ui, text, None);
}

fn field_label_help(ui: &mut egui::Ui, text: &str, help: Option<&str>) {
    if let Some(tip) = help {
        ui.horizontal(|ui| {
            ui.add_sized([140.0, 26.0], egui::Label::new(
                egui::RichText::new(text).size(15.0)
            ));
            widgets::help_icon(ui, tip);
        });
    } else {
        ui.add_sized([160.0, 26.0], egui::Label::new(
            egui::RichText::new(text).size(15.0)
        ));
    }
}

fn field_value_or_spinner(ui: &mut egui::Ui, text: &str, uuid: Uuid, state: &AppState) {
    if state.is_pending(&uuid) {
        ui.add(egui::Spinner::new().size(widgets::SPINNER_SIZE));
    } else {
        ui.label(egui::RichText::new(text).size(15.0).monospace());
    }
}
