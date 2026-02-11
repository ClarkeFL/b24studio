use eframe::egui;
use crate::state::{AppState, CalSubTab};
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::uuids;
use crate::protocol::codec;
use crate::protocol::calibration;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    // Sub-tab selector
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.ui.cal_sub_tab, CalSubTab::AutoCal, "Auto Calibration");
        ui.selectable_value(&mut state.ui.cal_sub_tab, CalSubTab::Multipoint, "Multipoint Calibration");
    });
    ui.separator();

    match state.ui.cal_sub_tab {
        CalSubTab::AutoCal => show_auto_cal(ui, state, ble),
        CalSubTab::Multipoint => show_multipoint(ui, state, ble),
    }
}

fn show_auto_cal(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    ui.heading(format!("Current Point : {}", state.ui.cal_wizard.current_point + 1));
    ui.add_space(8.0);

    // Low Value
    ui.horizontal(|ui| {
        ui.label("Low Value:");
        ui.add(
            egui::TextEdit::singleline(&mut state.ui.cal_wizard.low_value)
                .desired_width(120.0),
        );
        if ui.button("Acquire Low Input").clicked() {
            // Read the current base value as the low input
            ble.send(BleCommand::ReadCharacteristic(uuids::char_base_value()));
            // The acquired value will be stored when the read event arrives
            // We mark that we're waiting for a low acquisition
            state.ui.cal_wizard.low_acquired = state.calibration.base_value;
        }
        if let Some(val) = state.ui.cal_wizard.low_acquired {
            ui.monospace(format!("{val}"));
        } else {
            ui.monospace("0");
        }
    });

    // High Value
    ui.horizontal(|ui| {
        ui.label("High Value:");
        ui.add(
            egui::TextEdit::singleline(&mut state.ui.cal_wizard.high_value)
                .desired_width(120.0),
        );
        if ui.button("Acquire High Input").clicked() {
            ble.send(BleCommand::ReadCharacteristic(uuids::char_base_value()));
            state.ui.cal_wizard.high_acquired = state.calibration.base_value;
        }
        if let Some(val) = state.ui.cal_wizard.high_acquired {
            ui.monospace(format!("{val}"));
        } else {
            ui.monospace("0");
        }
    });

    ui.add_space(8.0);

    // Compute and display gain/offset if both acquired
    if let (Some(low_base), Some(high_base)) =
        (state.ui.cal_wizard.low_acquired, state.ui.cal_wizard.high_acquired)
    {
        let low_target: f32 = state.ui.cal_wizard.low_value.parse().unwrap_or(0.0);
        let high_target: f32 = state.ui.cal_wizard.high_value.parse().unwrap_or(0.0);

        if let Some((gain, offset)) =
            calibration::two_point_calibration(low_base, high_base, low_target, high_target)
        {
            ui.label(format!("Computed Gain: {gain:.6}"));
            ui.label(format!("Computed Offset: {offset:.6}"));

            ui.add_space(8.0);

            if ui.button("Apply Calibration").clicked() {
                // Write gain
                ble.send(BleCommand::WriteCharacteristic {
                    uuid: uuids::char_data_gain(),
                    data: codec::encode_f32_be(gain),
                });
                // Write offset
                ble.send(BleCommand::WriteCharacteristic {
                    uuid: uuids::char_data_offset(),
                    data: codec::encode_f32_be(offset),
                });
            }
        }
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Reset").clicked() {
            state.ui.cal_wizard.low_acquired = None;
            state.ui.cal_wizard.high_acquired = None;
            state.ui.cal_wizard.low_value.clear();
            state.ui.cal_wizard.high_value.clear();
            state.ui.cal_wizard.current_point = 0;
        }
        if ui.button("Add Point").clicked() {
            state.ui.cal_wizard.current_point += 1;
            // Copy high value to low value for next point
            state.ui.cal_wizard.low_value = state.ui.cal_wizard.high_value.clone();
            state.ui.cal_wizard.low_acquired = state.ui.cal_wizard.high_acquired;
            state.ui.cal_wizard.high_value.clear();
            state.ui.cal_wizard.high_acquired = None;
        }
    });

    ui.add_space(16.0);
    ui.separator();

    // Help text
    ui.label("Low Value  - Enter the known weight/engineering value for the applied low input.");
    ui.label("Acquire Low Input - Apply the low input and click Acquire once settled.");
    ui.label("High Value - Enter the known weight/engineering value for the applied high input.");
    ui.label("Acquire High Input - Apply the high input and click Acquire once settled.");
    ui.label("Add Point - Saves the current point; High Value becomes the next Low Value.");
}

fn show_multipoint(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    ui.heading("Linearisation Table");

    ui.horizontal(|ui| {
        if ui.button("Refresh").clicked() {
            // Read lin_points first, then we'll read the table
            ble.send(BleCommand::ReadCharacteristic(uuids::char_lin_points()));
        }
        ui.label(format!(
            "Linearisation Points: {}",
            state.calibration.lin_points.unwrap_or(0)
        ));
    });

    ui.add_space(8.0);

    // Table header
    egui::Grid::new("lin_table")
        .num_columns(5)
        .spacing([16.0, 4.0])
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
                // Show placeholder rows
                let max_rows = 15;
                for i in 1..=max_rows {
                    ui.label(format!("{i}"));
                    ui.label("-");
                    ui.label("-");
                    ui.label("-");
                    ui.label("-");
                    ui.end_row();
                }
            } else {
                for entry in &state.calibration.linearisation_table {
                    ui.label(format!("{}", entry.index));
                    ui.label(format!("{}", entry.valid_from));
                    ui.label(format!("{}", entry.gain));
                    ui.label(format!("{}", entry.offset));
                    ui.label(format!("{}", entry.valid_to));
                    ui.end_row();
                }
            }
        });
}
