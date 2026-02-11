use eframe::egui;
use crate::state::AppState;
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::uuids;
use crate::protocol::types::{DeviceAction, AdvancedParam};
use crate::ui::widgets::*;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── Calibration Registers ──────────────────────────────
        ui.heading("Calibration Registers");

        ui.horizontal(|ui| {
            if ui.button("Read All Calibration").clicked() {
                ble.send(BleCommand::ReadAll(uuids::all_calibration_uuids()));
            }
        });
        ui.separator();

        egui::Grid::new("cal_grid")
            .num_columns(1)
            .spacing([0.0, 2.0])
            .show(ui, |ui| {
                register_row_u8(
                    ui, "Sensitivity Range",
                    state.calibration.sensitivity_range,
                    &mut state.ui.edit.sensitivity_range,
                    uuids::char_sens_range, ble, false,
                );
                ui.end_row();

                register_row_f32(
                    ui, "Coefficient (@Index)",
                    state.calibration.coefficient,
                    &mut String::new(),
                    uuids::char_coeff_at_idx, ble, true,
                );
                ui.end_row();

                register_row_u8(
                    ui, "Linearisation Index",
                    state.calibration.lin_index,
                    &mut state.ui.edit.lin_index,
                    uuids::char_lin_index, ble, false,
                );
                ui.end_row();

                register_row_u8(
                    ui, "Linearisation Repeat",
                    state.calibration.lin_repeat,
                    &mut state.ui.edit.lin_repeat,
                    uuids::char_lin_repeat, ble, false,
                );
                ui.end_row();

                register_row_u8(
                    ui, "Linearisation Points",
                    state.calibration.lin_points,
                    &mut state.ui.edit.lin_points,
                    uuids::char_lin_points, ble, false,
                );
                ui.end_row();

                register_row_f32(
                    ui, "Base Value",
                    state.calibration.base_value,
                    &mut String::new(),
                    uuids::char_base_value, ble, true,
                );
                ui.end_row();

                register_row_u8(
                    ui, "Base Units",
                    state.calibration.base_units,
                    &mut String::new(),
                    uuids::char_base_units, ble, true,
                );
                ui.end_row();

                register_row_f32(
                    ui, "Data Gain",
                    state.calibration.data_gain,
                    &mut state.ui.edit.data_gain,
                    uuids::char_data_gain, ble, false,
                );
                ui.end_row();

                register_row_f32(
                    ui, "Data Offset",
                    state.calibration.data_offset,
                    &mut state.ui.edit.data_offset,
                    uuids::char_data_offset, ble, false,
                );
                ui.end_row();

                register_row_u32(
                    ui, "Calibration PIN",
                    state.calibration.cal_pin,
                    &mut state.ui.edit.cal_pin,
                    uuids::char_cal_pin, ble, false,
                );
                ui.end_row();

                register_row_u8(
                    ui, "Calibration Units",
                    state.calibration.cal_units,
                    &mut state.ui.edit.cal_units,
                    uuids::char_cal_units, ble, false,
                );
                ui.end_row();
            });

        ui.add_space(16.0);

        // ── Advanced Registers ─────────────────────────────────
        ui.heading("Advanced Registers");
        ui.separator();

        egui::Grid::new("adv_grid")
            .num_columns(1)
            .spacing([0.0, 2.0])
            .show(ui, |ui| {
                for param in AdvancedParam::ALL {
                    let index = param.index();
                    let value_str = state.advanced.get_f32(index)
                        .map(|v| format!("{v}"))
                        .or_else(|| state.advanced.get_u8(index).map(|v| format!("{v}")))
                        .unwrap_or_else(|| "--".to_string());

                    ui.horizontal(|ui| {
                        ui.label(format!("{:<24}", param.label()));
                        ui.monospace(format!("{value_str:<14}"));
                        if ui.button("Read").clicked() {
                            ble.send(BleCommand::ReadAdvanced { index });
                        }
                        // Write for non-read-only params
                        if *param != AdvancedParam::PeakValue && *param != AdvancedParam::TroughValue {
                            if ui.button("Write").clicked() {
                                // For write, parse from the adv_data edit buffer
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

        // Shared edit buffer for advanced writes
        ui.horizontal(|ui| {
            ui.label("Advanced Write Value:");
            ui.add(egui::TextEdit::singleline(&mut state.ui.edit.adv_data).desired_width(100.0));
        });

        ui.add_space(16.0);

        // ── Actions ────────────────────────────────────────────
        ui.heading("Actions");
        ui.separator();

        ui.horizontal_wrapped(|ui| {
            let actions = [
                DeviceAction::ShuntCalOn,
                DeviceAction::ShuntCalOff,
                DeviceAction::ResetPeakTrough,
                DeviceAction::Tare,
                DeviceAction::ResetTare,
                DeviceAction::Reboot,
                DeviceAction::RestoreEepromDefaults,
            ];
            for action in actions {
                if ui.button(action.label()).clicked() {
                    ble.send(BleCommand::ExecuteAction(action));
                }
            }
        });
    });
}
