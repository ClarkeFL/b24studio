use eframe::egui;
use uuid::Uuid;
use crate::state::{AppState, CalSubTab};
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::{uuids, codec};
use crate::protocol::types::{DeviceAction, AdvancedParam, SensitivityRange, DataUnits};
use crate::protocol::calibration;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    ui.add_space(4.0);

    // Sub-tab bar
    ui.horizontal(|ui| {
        ui.heading("Calibration");
        ui.add_space(16.0);
        ui.selectable_value(&mut state.ui.cal_sub_tab, CalSubTab::AutoCal, "Registers & Auto Cal");
        ui.selectable_value(&mut state.ui.cal_sub_tab, CalSubTab::Multipoint, "Advanced & Linearisation");
    });
    ui.separator();

    match state.ui.cal_sub_tab {
        CalSubTab::AutoCal => show_registers_and_autocal(ui, state, ble),
        CalSubTab::Multipoint => show_advanced_and_lin(ui, state, ble),
    }
}

// ── Helpers ────────────────────────────────────────────────────────

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

fn send_tracked(state: &mut AppState, ble: &BleHandle, cmd: BleCommand) {
    state.track_send(&cmd);
    ble.send(cmd);
}

fn value_or_spinner(ui: &mut egui::Ui, text: &str, uuid: Uuid, state: &AppState) {
    if state.is_pending(&uuid) {
        ui.add(egui::Spinner::new().size(14.0));
    } else {
        ui.monospace(text);
    }
}

fn adv_value_or_spinner(ui: &mut egui::Ui, text: &str, index: u8, state: &AppState) {
    if state.is_adv_pending(index) {
        ui.add(egui::Spinner::new().size(14.0));
    } else {
        ui.monospace(text);
    }
}

// ── AutoCal Sub-tab ────────────────────────────────────────────────

fn show_registers_and_autocal(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);

        // ── Row 1: Calibration Registers (left) | Auto Cal Wizard (right)
        ui.columns(2, |cols| {
            // LEFT: Calibration Registers
            cols[0].horizontal(|ui| {
                section_header(ui, "Calibration Registers");
                ui.add_space(16.0);
                if ui.button("Read All").clicked() {
                    send_tracked(state, ble, BleCommand::ReadAll(uuids::all_calibration_uuids()));
                }
            });

            let edit_w = 120.0;
            let label_w = 160.0;

            egui::Grid::new("cal_registers")
                .num_columns(3)
                .spacing([12.0, 8.0])
                .min_col_width(40.0)
                .show(&mut cols[0], |ui| {
                    // Sensitivity Range
                    ui.add_sized([label_w, 26.0], egui::Label::new("Sensitivity Range"));
                    let sr_str = state.calibration.sensitivity_range
                        .and_then(SensitivityRange::from_byte)
                        .map(|s| s.label().to_string())
                        .unwrap_or_else(|| state.calibration.sensitivity_range
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| "--".to_string()));
                    value_or_spinner(ui, &sr_str, uuids::char_sens_range(), state);
                    ui.add_sized([edit_w, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.sensitivity_range)
                        .hint_text("0-3"));
                    ui.end_row();

                    // Data Gain
                    ui.add_sized([label_w, 26.0], egui::Label::new("Data Gain"));
                    let dg = state.calibration.data_gain
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &dg, uuids::char_data_gain(), state);
                    ui.add_sized([edit_w, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.data_gain));
                    ui.end_row();

                    // Data Offset
                    ui.add_sized([label_w, 26.0], egui::Label::new("Data Offset"));
                    let do_ = state.calibration.data_offset
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &do_, uuids::char_data_offset(), state);
                    ui.add_sized([edit_w, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.data_offset));
                    ui.end_row();

                    // Coefficient (read-only)
                    ui.add_sized([label_w, 26.0], egui::Label::new("Coefficient (@ Idx)"));
                    let cf = state.calibration.coefficient
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &cf, uuids::char_coeff_at_idx(), state);
                    ui.label(""); // no edit
                    ui.end_row();

                    // Base Value (read-only)
                    ui.add_sized([label_w, 26.0], egui::Label::new("Base Value"));
                    let bv = state.calibration.base_value
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &bv, uuids::char_base_value(), state);
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
                    value_or_spinner(ui, &bu, uuids::char_base_units(), state);
                    ui.label("");
                    ui.end_row();

                    // Cal PIN
                    ui.add_sized([label_w, 26.0], egui::Label::new("Calibration PIN"));
                    let cp = state.calibration.cal_pin
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &cp, uuids::char_cal_pin(), state);
                    ui.add_sized([edit_w, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.cal_pin));
                    ui.end_row();

                    // Cal Units (dropdown)
                    ui.add_sized([label_w, 26.0], egui::Label::new("Calibration Units"));
                    let cu = state.calibration.cal_units
                        .map(|u| {
                            let du = DataUnits::from_byte(u);
                            format!("{} (0x{:02X})", du.label(), u)
                        })
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &cu, uuids::char_cal_units(), state);

                    // Dropdown for selecting calibration units
                    let selected_label = if state.ui.edit.cal_units_display.is_empty() {
                        "Select unit...".to_string()
                    } else if let Ok(byte_val) = state.ui.edit.cal_units_display.parse::<u8>() {
                        DataUnits::from_byte(byte_val).dropdown_label()
                    } else {
                        "Select unit...".to_string()
                    };
                    egui::ComboBox::from_id_salt("cal_units_combo")
                        .selected_text(&selected_label)
                        .width(edit_w)
                        .show_ui(ui, |ui| {
                            // Option to clear selection
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
                });

            cols[0].add_space(8.0);

            // Save button for calibration registers
            if cols[0].add_sized([130.0, 32.0], egui::Button::new(
                egui::RichText::new("Save Registers").size(15.0)
            ).fill(egui::Color32::from_rgb(40, 120, 60))).clicked() {
                save_cal_registers(state, ble);
            }

            // RIGHT: Auto Calibration Wizard
            section_header(&mut cols[1], "Auto Calibration Wizard");

            cols[1].label(
                egui::RichText::new(format!("Point {}", state.ui.cal_wizard.current_point + 1))
                    .size(15.0)
                    .strong(),
            );
            cols[1].add_space(4.0);

            egui::Grid::new("autocal_grid")
                .num_columns(4)
                .spacing([12.0, 8.0])
                .min_col_width(60.0)
                .show(&mut cols[1], |ui| {
                    ui.label("Low Value:");
                    ui.add_sized([120.0, 30.0], egui::TextEdit::singleline(&mut state.ui.cal_wizard.low_value)
                        .hint_text("known low"));
                    if ui.button("Acquire Low").clicked() {
                        send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_base_value()));
                        state.ui.cal_wizard.low_acquired = state.calibration.base_value;
                    }
                    let low_str = state.ui.cal_wizard.low_acquired
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "0".to_string());
                    ui.monospace(&low_str);
                    ui.end_row();

                    ui.label("High Value:");
                    ui.add_sized([120.0, 30.0], egui::TextEdit::singleline(&mut state.ui.cal_wizard.high_value)
                        .hint_text("known high"));
                    if ui.button("Acquire High").clicked() {
                        send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_base_value()));
                        state.ui.cal_wizard.high_acquired = state.calibration.base_value;
                    }
                    let high_str = state.ui.cal_wizard.high_acquired
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "0".to_string());
                    ui.monospace(&high_str);
                    ui.end_row();
                });

            cols[1].add_space(8.0);

            // Computed gain/offset
            if let (Some(low_base), Some(high_base)) =
                (state.ui.cal_wizard.low_acquired, state.ui.cal_wizard.high_acquired)
            {
                let low_target: f32 = state.ui.cal_wizard.low_value.parse().unwrap_or(0.0);
                let high_target: f32 = state.ui.cal_wizard.high_value.parse().unwrap_or(0.0);

                if let Some((gain, offset)) =
                    calibration::two_point_calibration(low_base, high_base, low_target, high_target)
                {
                    cols[1].horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Gain: {gain:.6}"))
                                .monospace().size(14.0),
                        );
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new(format!("Offset: {offset:.6}"))
                                .monospace().size(14.0),
                        );
                        ui.add_space(16.0);
                        if ui.add_sized([100.0, 28.0], egui::Button::new("Apply")).clicked() {
                            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                                uuid: uuids::char_data_gain(),
                                data: codec::encode_f32_be(gain),
                            });
                            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                                uuid: uuids::char_data_offset(),
                                data: codec::encode_f32_be(offset),
                            });
                        }
                    });
                }
            }

            cols[1].add_space(8.0);

            cols[1].horizontal(|ui| {
                if ui.button("Reset Wizard").clicked() {
                    state.ui.cal_wizard.low_acquired = None;
                    state.ui.cal_wizard.high_acquired = None;
                    state.ui.cal_wizard.low_value.clear();
                    state.ui.cal_wizard.high_value.clear();
                    state.ui.cal_wizard.current_point = 0;
                }
                if ui.button("Add Point").clicked() {
                    state.ui.cal_wizard.current_point += 1;
                    state.ui.cal_wizard.low_value = state.ui.cal_wizard.high_value.clone();
                    state.ui.cal_wizard.low_acquired = state.ui.cal_wizard.high_acquired;
                    state.ui.cal_wizard.high_value.clear();
                    state.ui.cal_wizard.high_acquired = None;
                }
            });
        });

        ui.add_space(16.0);

        // ── Device Actions (full width) ────────────────────────────
        section_header(ui, "Device Actions");

        ui.horizontal_wrapped(|ui| {
            let actions = [
                (DeviceAction::Tare, egui::Color32::from_rgb(50, 100, 180)),
                (DeviceAction::ResetTare, egui::Color32::from_rgb(80, 80, 100)),
                (DeviceAction::ShuntCalOn, egui::Color32::from_rgb(50, 130, 80)),
                (DeviceAction::ShuntCalOff, egui::Color32::from_rgb(80, 80, 100)),
                (DeviceAction::ResetPeakTrough, egui::Color32::from_rgb(180, 130, 50)),
                (DeviceAction::Reboot, egui::Color32::from_rgb(180, 100, 50)),
                (DeviceAction::RestoreEepromDefaults, egui::Color32::from_rgb(180, 50, 50)),
            ];
            for (action, color) in actions {
                if ui.add_sized(
                    [150.0, 32.0],
                    egui::Button::new(egui::RichText::new(action.label()).size(14.0)).fill(color),
                ).clicked() {
                    ble.send(BleCommand::ExecuteAction(action));
                }
            }
        });
    });
}

// ── Save calibration registers ─────────────────────────────────────

fn save_cal_registers(state: &mut AppState, ble: &BleHandle) {
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

    // Re-read all cal registers
    send_tracked(state, ble, BleCommand::ReadAll(uuids::all_calibration_uuids()));

    // Clear edit buffers
    state.ui.edit.sensitivity_range.clear();
    state.ui.edit.data_gain.clear();
    state.ui.edit.data_offset.clear();
    state.ui.edit.cal_pin.clear();
    state.ui.edit.cal_units_display.clear();
}

// ── Advanced & Linearisation Sub-tab ───────────────────────────────

fn show_advanced_and_lin(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);

        // ── Row: Advanced Params (left) | Linearisation (right)
        ui.columns(2, |cols| {
            // LEFT: Advanced Parameters
            section_header(&mut cols[0], "Advanced Parameters");

            cols[0].horizontal(|ui| {
                if ui.button("Read All Advanced").clicked() {
                    for param in AdvancedParam::ALL {
                        send_tracked(state, ble, BleCommand::ReadAdvanced { index: param.index() });
                    }
                }
            });
            cols[0].add_space(4.0);

            egui::Grid::new("adv_params")
                .num_columns(4)
                .spacing([12.0, 6.0])
                .min_col_width(40.0)
                .show(&mut cols[0], |ui| {
                    ui.strong("Parameter");
                    ui.strong("Value");
                    ui.strong("Edit");
                    ui.strong("");
                    ui.end_row();

                    for param in AdvancedParam::ALL {
                        let index = param.index();
                        let is_readonly = *param == AdvancedParam::PeakValue
                            || *param == AdvancedParam::TroughValue;

                        ui.add_sized([160.0, 26.0], egui::Label::new(param.label()));

                        let value_str = state.advanced.get_f32(index)
                            .map(|v| format!("{v}"))
                            .or_else(|| state.advanced.get_u8(index).map(|v| format!("{v}")))
                            .unwrap_or_else(|| "--".to_string());
                        adv_value_or_spinner(ui, &value_str, index, state);

                        if is_readonly {
                            ui.label("");
                        } else {
                            ui.add_sized([100.0, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.adv_data));
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Read").clicked() {
                                send_tracked(state, ble, BleCommand::ReadAdvanced { index });
                            }
                            if !is_readonly {
                                if ui.button("Write").clicked() {
                                    if let Ok(val) = state.ui.edit.adv_data.parse::<f32>() {
                                        send_tracked(state, ble, BleCommand::WriteAdvanced {
                                            index,
                                            data: val.to_be_bytes().to_vec(),
                                        });
                                    } else if let Ok(val) = state.ui.edit.adv_data.parse::<u8>() {
                                        send_tracked(state, ble, BleCommand::WriteAdvanced {
                                            index,
                                            data: vec![val],
                                        });
                                    }
                                }
                            }
                        });
                        ui.end_row();
                    }
                });

            // RIGHT: Linearisation Table
            section_header(&mut cols[1], "Linearisation Table");

            cols[1].horizontal(|ui| {
                if ui.button("Refresh Table").clicked() {
                    send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_lin_points()));
                }
                ui.add_space(8.0);
                ui.label(format!(
                    "Points: {}",
                    state.calibration.lin_points.unwrap_or(0)
                ));
            });

            cols[1].add_space(4.0);

            // Linearisation controls
            egui::Grid::new("lin_controls")
                .num_columns(4)
                .spacing([12.0, 8.0])
                .show(&mut cols[1], |ui| {
                    ui.label("Lin Index:");
                    ui.add_sized([60.0, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.lin_index));
                    let li_str = state.calibration.lin_index
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &li_str, uuids::char_lin_index(), state);
                    ui.horizontal(|ui| {
                        if ui.button("Read").clicked() {
                            send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_lin_index()));
                        }
                        if ui.button("Write").clicked() {
                            if let Ok(v) = state.ui.edit.lin_index.parse::<u8>() {
                                send_tracked(state, ble, BleCommand::WriteCharacteristic {
                                    uuid: uuids::char_lin_index(), data: codec::encode_u8(v),
                                });
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Lin Repeat:");
                    ui.add_sized([60.0, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.lin_repeat));
                    let lr_str = state.calibration.lin_repeat
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &lr_str, uuids::char_lin_repeat(), state);
                    ui.horizontal(|ui| {
                        if ui.button("Read").clicked() {
                            send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_lin_repeat()));
                        }
                        if ui.button("Write").clicked() {
                            if let Ok(v) = state.ui.edit.lin_repeat.parse::<u8>() {
                                send_tracked(state, ble, BleCommand::WriteCharacteristic {
                                    uuid: uuids::char_lin_repeat(), data: codec::encode_u8(v),
                                });
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Lin Points:");
                    ui.add_sized([60.0, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.lin_points));
                    let lp_str = state.calibration.lin_points
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &lp_str, uuids::char_lin_points(), state);
                    ui.horizontal(|ui| {
                        if ui.button("Read").clicked() {
                            send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_lin_points()));
                        }
                        if ui.button("Write").clicked() {
                            if let Ok(v) = state.ui.edit.lin_points.parse::<u8>() {
                                send_tracked(state, ble, BleCommand::WriteCharacteristic {
                                    uuid: uuids::char_lin_points(), data: codec::encode_u8(v),
                                });
                            }
                        }
                    });
                    ui.end_row();
                });

            cols[1].add_space(8.0);

            // Linearisation data table
            egui::Grid::new("lin_table")
                .num_columns(5)
                .spacing([16.0, 4.0])
                .striped(true)
                .min_col_width(70.0)
                .show(&mut cols[1], |ui| {
                    ui.strong("Index");
                    ui.strong("Valid From");
                    ui.strong("Gain");
                    ui.strong("Offset");
                    ui.strong("Valid To");
                    ui.end_row();

                    if state.calibration.linearisation_table.is_empty() {
                        for i in 1..=15 {
                            ui.label(format!("{i}"));
                            ui.monospace("-");
                            ui.monospace("-");
                            ui.monospace("-");
                            ui.monospace("-");
                            ui.end_row();
                        }
                    } else {
                        for entry in &state.calibration.linearisation_table {
                            ui.label(format!("{}", entry.index));
                            ui.monospace(format!("{}", entry.valid_from));
                            ui.monospace(format!("{}", entry.gain));
                            ui.monospace(format!("{}", entry.offset));
                            ui.monospace(format!("{}", entry.valid_to));
                            ui.end_row();
                        }
                    }
                });
        });
    });
}
