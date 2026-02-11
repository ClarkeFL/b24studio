use eframe::egui;
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

fn show_registers_and_autocal(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        let grid_spacing = [20.0, 6.0];
        let label_w = 160.0;
        let edit_w = 120.0;

        // ── Calibration Registers ───────────────────────────────
        ui.horizontal(|ui| {
            section_header(ui, "Calibration Registers");
            ui.add_space(16.0);
            if ui.button("Read All").clicked() {
                ble.send(BleCommand::ReadAll(uuids::all_calibration_uuids()));
            }
        });

        egui::Grid::new("cal_registers")
            .num_columns(5)
            .spacing(grid_spacing)
            .min_col_width(40.0)
            .show(ui, |ui| {
                // Sensitivity Range
                ui.add_sized([label_w, 20.0], egui::Label::new("Sensitivity Range"));
                let sr_str = state.calibration.sensitivity_range
                    .and_then(SensitivityRange::from_byte)
                    .map(|s| s.label().to_string())
                    .unwrap_or_else(|| state.calibration.sensitivity_range
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string()));
                ui.monospace(&sr_str);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.sensitivity_range)
                    .desired_width(edit_w).hint_text("0-3"));
                if ui.button("Read").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_sens_range()));
                }
                if ui.button("Write").clicked() {
                    if let Ok(v) = state.ui.edit.sensitivity_range.parse::<u8>() {
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_sens_range(),
                            data: codec::encode_u8(v),
                        });
                    }
                }
                ui.end_row();

                // Data Gain
                ui.add_sized([label_w, 20.0], egui::Label::new("Data Gain"));
                let dg = state.calibration.data_gain
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "--".to_string());
                ui.monospace(&dg);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.data_gain)
                    .desired_width(edit_w));
                if ui.button("Read").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_data_gain()));
                }
                if ui.button("Write").clicked() {
                    if let Ok(v) = state.ui.edit.data_gain.parse::<f32>() {
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_data_gain(),
                            data: codec::encode_f32_be(v),
                        });
                    }
                }
                ui.end_row();

                // Data Offset
                ui.add_sized([label_w, 20.0], egui::Label::new("Data Offset"));
                let do_ = state.calibration.data_offset
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "--".to_string());
                ui.monospace(&do_);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.data_offset)
                    .desired_width(edit_w));
                if ui.button("Read").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_data_offset()));
                }
                if ui.button("Write").clicked() {
                    if let Ok(v) = state.ui.edit.data_offset.parse::<f32>() {
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_data_offset(),
                            data: codec::encode_f32_be(v),
                        });
                    }
                }
                ui.end_row();

                // Coefficient @ Index (read-only)
                ui.add_sized([label_w, 20.0], egui::Label::new("Coefficient (@ Index)"));
                let cf = state.calibration.coefficient
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "--".to_string());
                ui.monospace(&cf);
                ui.label("");
                if ui.button("Read").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_coeff_at_idx()));
                }
                ui.label("");
                ui.end_row();

                // Base Value (read-only)
                ui.add_sized([label_w, 20.0], egui::Label::new("Base Value"));
                let bv = state.calibration.base_value
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "--".to_string());
                ui.monospace(&bv);
                ui.label("");
                if ui.button("Read").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_base_value()));
                }
                ui.label("");
                ui.end_row();

                // Base Units (read-only)
                ui.add_sized([label_w, 20.0], egui::Label::new("Base Units"));
                let bu = state.calibration.base_units
                    .map(|u| {
                        let du = DataUnits::from_byte(u);
                        format!("{} ({})", du.label(), u)
                    })
                    .unwrap_or_else(|| "--".to_string());
                ui.monospace(&bu);
                ui.label("");
                if ui.button("Read").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_base_units()));
                }
                ui.label("");
                ui.end_row();

                // Calibration PIN
                ui.add_sized([label_w, 20.0], egui::Label::new("Calibration PIN"));
                let cp = state.calibration.cal_pin
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "--".to_string());
                ui.monospace(&cp);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.cal_pin)
                    .desired_width(edit_w));
                if ui.button("Read").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_cal_pin()));
                }
                if ui.button("Write").clicked() {
                    if let Ok(v) = state.ui.edit.cal_pin.parse::<u32>() {
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_cal_pin(),
                            data: codec::encode_u32_be(v),
                        });
                    }
                }
                ui.end_row();

                // Cal Units
                ui.add_sized([label_w, 20.0], egui::Label::new("Calibration Units"));
                let cu = state.calibration.cal_units
                    .map(|u| {
                        let du = DataUnits::from_byte(u);
                        format!("{} ({})", du.label(), u)
                    })
                    .unwrap_or_else(|| "--".to_string());
                ui.monospace(&cu);
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.cal_units_display)
                    .desired_width(edit_w).hint_text("unit byte"));
                if ui.button("Read").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_cal_units()));
                }
                if ui.button("Write").clicked() {
                    if let Ok(v) = state.ui.edit.cal_units_display.parse::<u8>() {
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_cal_units(),
                            data: codec::encode_u8(v),
                        });
                    }
                }
                ui.end_row();
            });

        ui.add_space(16.0);

        // ── Auto Calibration Wizard ─────────────────────────────
        section_header(ui, "Auto Calibration Wizard");

        ui.label(
            egui::RichText::new(format!("Point {}", state.ui.cal_wizard.current_point + 1))
                .size(14.0)
                .strong(),
        );
        ui.add_space(4.0);

        egui::Grid::new("autocal_grid")
            .num_columns(4)
            .spacing([16.0, 8.0])
            .min_col_width(60.0)
            .show(ui, |ui| {
                // Low Value
                ui.label("Low Value:");
                ui.add(egui::TextEdit::singleline(&mut state.ui.cal_wizard.low_value)
                    .desired_width(120.0).hint_text("known low"));
                if ui.button("Acquire Low").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_base_value()));
                    state.ui.cal_wizard.low_acquired = state.calibration.base_value;
                }
                let low_str = state.ui.cal_wizard.low_acquired
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "0".to_string());
                ui.monospace(&low_str);
                ui.end_row();

                // High Value
                ui.label("High Value:");
                ui.add(egui::TextEdit::singleline(&mut state.ui.cal_wizard.high_value)
                    .desired_width(120.0).hint_text("known high"));
                if ui.button("Acquire High").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_base_value()));
                    state.ui.cal_wizard.high_acquired = state.calibration.base_value;
                }
                let high_str = state.ui.cal_wizard.high_acquired
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "0".to_string());
                ui.monospace(&high_str);
                ui.end_row();
            });

        ui.add_space(8.0);

        // Computed gain/offset
        if let (Some(low_base), Some(high_base)) =
            (state.ui.cal_wizard.low_acquired, state.ui.cal_wizard.high_acquired)
        {
            let low_target: f32 = state.ui.cal_wizard.low_value.parse().unwrap_or(0.0);
            let high_target: f32 = state.ui.cal_wizard.high_value.parse().unwrap_or(0.0);

            if let Some((gain, offset)) =
                calibration::two_point_calibration(low_base, high_base, low_target, high_target)
            {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Gain: {gain:.6}"))
                            .monospace()
                            .size(14.0),
                    );
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new(format!("Offset: {offset:.6}"))
                            .monospace()
                            .size(14.0),
                    );
                    ui.add_space(24.0);
                    if ui.add_sized([120.0, 28.0], egui::Button::new("Apply")).clicked() {
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_data_gain(),
                            data: codec::encode_f32_be(gain),
                        });
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_data_offset(),
                            data: codec::encode_f32_be(offset),
                        });
                    }
                });
            }
        }

        ui.add_space(8.0);

        ui.horizontal(|ui| {
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

        ui.add_space(16.0);

        // ── Device Actions ──────────────────────────────────────
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
                    [150.0, 28.0],
                    egui::Button::new(action.label()).fill(color),
                ).clicked() {
                    ble.send(BleCommand::ExecuteAction(action));
                }
            }
        });
    });
}

fn show_advanced_and_lin(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── Advanced Parameters ─────────────────────────────────
        section_header(ui, "Advanced Parameters");

        ui.horizontal(|ui| {
            if ui.button("Read All Advanced").clicked() {
                for param in AdvancedParam::ALL {
                    ble.send(BleCommand::ReadAdvanced { index: param.index() });
                }
            }
        });
        ui.add_space(4.0);

        egui::Grid::new("adv_params")
            .num_columns(4)
            .spacing([20.0, 4.0])
            .min_col_width(40.0)
            .show(ui, |ui| {
                ui.strong("Parameter");
                ui.strong("Value");
                ui.strong("Edit");
                ui.strong("");
                ui.end_row();

                for param in AdvancedParam::ALL {
                    let index = param.index();
                    let is_readonly = *param == AdvancedParam::PeakValue
                        || *param == AdvancedParam::TroughValue;

                    ui.add_sized([180.0, 20.0], egui::Label::new(param.label()));

                    let value_str = state.advanced.get_f32(index)
                        .map(|v| format!("{v}"))
                        .or_else(|| state.advanced.get_u8(index).map(|v| format!("{v}")))
                        .unwrap_or_else(|| "--".to_string());
                    ui.monospace(&value_str);

                    if is_readonly {
                        ui.label("");
                    } else {
                        // Each advanced param gets its own edit buffer stored in adv_data
                        // For simplicity, use a shared buffer
                        ui.add(egui::TextEdit::singleline(&mut state.ui.edit.adv_data)
                            .desired_width(100.0));
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Read").clicked() {
                            ble.send(BleCommand::ReadAdvanced { index });
                        }
                        if !is_readonly {
                            if ui.button("Write").clicked() {
                                if let Ok(val) = state.ui.edit.adv_data.parse::<f32>() {
                                    ble.send(BleCommand::WriteAdvanced {
                                        index,
                                        data: val.to_be_bytes().to_vec(),
                                    });
                                } else if let Ok(val) = state.ui.edit.adv_data.parse::<u8>() {
                                    ble.send(BleCommand::WriteAdvanced {
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

        ui.add_space(16.0);

        // ── Linearisation Table ─────────────────────────────────
        section_header(ui, "Linearisation Table");

        ui.horizontal(|ui| {
            if ui.button("Refresh Table").clicked() {
                ble.send(BleCommand::ReadCharacteristic(uuids::char_lin_points()));
            }
            ui.add_space(8.0);
            ui.label(format!(
                "Points: {}",
                state.calibration.lin_points.unwrap_or(0)
            ));
        });

        ui.add_space(4.0);

        // Linearisation index/repeat controls
        egui::Grid::new("lin_controls")
            .num_columns(5)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                ui.label("Lin Index:");
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.lin_index)
                    .desired_width(60.0));
                if ui.button("Read").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_lin_index()));
                }
                if ui.button("Write").clicked() {
                    if let Ok(v) = state.ui.edit.lin_index.parse::<u8>() {
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_lin_index(),
                            data: codec::encode_u8(v),
                        });
                    }
                }
                let li_str = state.calibration.lin_index
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "--".to_string());
                ui.monospace(&li_str);
                ui.end_row();

                ui.label("Lin Repeat:");
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.lin_repeat)
                    .desired_width(60.0));
                if ui.button("Read").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_lin_repeat()));
                }
                if ui.button("Write").clicked() {
                    if let Ok(v) = state.ui.edit.lin_repeat.parse::<u8>() {
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_lin_repeat(),
                            data: codec::encode_u8(v),
                        });
                    }
                }
                let lr_str = state.calibration.lin_repeat
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "--".to_string());
                ui.monospace(&lr_str);
                ui.end_row();

                ui.label("Lin Points:");
                ui.add(egui::TextEdit::singleline(&mut state.ui.edit.lin_points)
                    .desired_width(60.0));
                if ui.button("Read").clicked() {
                    ble.send(BleCommand::ReadCharacteristic(uuids::char_lin_points()));
                }
                if ui.button("Write").clicked() {
                    if let Ok(v) = state.ui.edit.lin_points.parse::<u8>() {
                        ble.send(BleCommand::WriteCharacteristic {
                            uuid: uuids::char_lin_points(),
                            data: codec::encode_u8(v),
                        });
                    }
                }
                let lp_str = state.calibration.lin_points
                    .map(|v| format!("{v}"))
                    .unwrap_or_else(|| "--".to_string());
                ui.monospace(&lp_str);
                ui.end_row();
            });

        ui.add_space(8.0);

        // Linearisation data table
        egui::Grid::new("lin_table")
            .num_columns(5)
            .spacing([20.0, 4.0])
            .striped(true)
            .min_col_width(80.0)
            .show(ui, |ui| {
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
}
