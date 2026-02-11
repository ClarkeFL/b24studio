use eframe::egui;
use crate::state::AppState;
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::uuids;
use crate::ui::widgets::*;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    ui.heading("Configuration Registers");

    ui.horizontal(|ui| {
        if ui.button("Read All Registers").clicked() {
            ble.send(BleCommand::ReadAll(uuids::all_config_uuids()));
        }
    });

    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("config_grid")
            .num_columns(1)
            .spacing([0.0, 2.0])
            .show(ui, |ui| {
                // Configuration PIN
                register_row_u32(
                    ui,
                    "Configuration PIN",
                    state.config.config_pin,
                    &mut state.ui.edit.config_pin,
                    uuids::char_config_pin,
                    ble,
                    false,
                );
                ui.end_row();

                // Data Rate
                register_row_u32(
                    ui,
                    "Data Rate (ms)",
                    state.config.data_rate,
                    &mut state.ui.edit.data_rate,
                    uuids::char_data_rate,
                    ble,
                    false,
                );
                ui.end_row();

                // Resolution
                register_row_u8(
                    ui,
                    "Resolution",
                    state.config.resolution,
                    &mut state.ui.edit.resolution,
                    uuids::char_resolution,
                    ble,
                    false,
                );
                ui.end_row();

                // Battery Threshold
                register_row_f32(
                    ui,
                    "Battery Threshold (V)",
                    state.config.battery_threshold,
                    &mut state.ui.edit.battery_threshold,
                    uuids::char_battery_thresh,
                    ble,
                    false,
                );
                ui.end_row();

                // View PIN
                register_row_string(
                    ui,
                    "View PIN",
                    state.config.view_pin.as_deref(),
                    &mut state.ui.edit.view_pin,
                    uuids::char_view_pin,
                    ble,
                    false,
                );
                ui.end_row();

                // Serial Number (read-only)
                register_row_string(
                    ui,
                    "Serial Number",
                    state.config.serial_number.as_deref(),
                    &mut String::new(),
                    uuids::char_serial_number,
                    ble,
                    true,
                );
                ui.end_row();

                // Data Tag
                register_row_string(
                    ui,
                    "Data Tag",
                    state.config.data_tag.as_deref(),
                    &mut state.ui.edit.data_tag,
                    uuids::char_data_tag,
                    ble,
                    false,
                );
                ui.end_row();

                // Battery Value (read-only)
                register_row_f32(
                    ui,
                    "Battery Value (V)",
                    state.config.battery_value,
                    &mut String::new(),
                    uuids::char_battery_value,
                    ble,
                    true,
                );
                ui.end_row();

                // System Zero
                register_row_f32(
                    ui,
                    "System Zero",
                    state.config.system_zero,
                    &mut state.ui.edit.system_zero,
                    uuids::char_system_zero,
                    ble,
                    false,
                );
                ui.end_row();

                // Model Name (read-only)
                register_row_string(
                    ui,
                    "Model",
                    state.config.model_name.as_deref(),
                    &mut String::new(),
                    uuids::char_model_name,
                    ble,
                    true,
                );
                ui.end_row();

                // Firmware Version (read-only)
                register_row_string(
                    ui,
                    "Firmware Version",
                    state.config.firmware_version.as_deref(),
                    &mut String::new(),
                    uuids::char_firmware_ver,
                    ble,
                    true,
                );
                ui.end_row();
            });
    });
}
