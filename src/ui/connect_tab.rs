use eframe::egui;
use crate::state::{AppState, ConnectionPhase};
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    ui.heading("Connect to B24 Device");
    ui.separator();

    // Connection controls
    ui.horizontal(|ui| {
        ui.label("Configuration PIN:");
        ui.add(
            egui::TextEdit::singleline(&mut state.connection.config_pin)
                .desired_width(80.0)
                .hint_text("9999"),
        );
    });

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        let is_scanning = state.connection.phase == ConnectionPhase::Scanning;
        let is_connected = state.connection.phase == ConnectionPhase::Connected;

        if !is_connected {
            if ui
                .button(if is_scanning { "Stop Scan" } else { "Start Scanning" })
                .clicked()
            {
                if is_scanning {
                    ble.send(BleCommand::StopScan);
                    state.connection.phase = ConnectionPhase::Disconnected;
                } else {
                    state.connection.scanned_devices.clear();
                    ble.send(BleCommand::StartScan);
                    state.connection.phase = ConnectionPhase::Scanning;
                }
            }
        }

        if is_connected {
            if ui
                .add(egui::Button::new("Disconnect").fill(egui::Color32::from_rgb(200, 60, 60)))
                .clicked()
            {
                ble.send(BleCommand::Disconnect);
                state.connection.phase = ConnectionPhase::Disconnected;
            }
        }
    });

    ui.add_space(8.0);

    // Device list
    if state.connection.scanned_devices.is_empty() {
        if state.connection.phase == ConnectionPhase::Scanning {
            ui.spinner();
            ui.label("Scanning for B24 devices...");
        } else {
            ui.label("No devices found. Click 'Start Scanning' to discover B24 modules.");
        }
    } else {
        ui.label(format!(
            "Found {} device(s):",
            state.connection.scanned_devices.len()
        ));
        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Grid::new("device_grid")
                    .num_columns(4)
                    .spacing([16.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Header
                        ui.strong("Name");
                        ui.strong("RSSI");
                        ui.strong("ID");
                        ui.strong("");
                        ui.end_row();

                        let mut connect_idx = None;

                        for (idx, device) in state.connection.scanned_devices.iter().enumerate() {
                            ui.label(&device.name);
                            ui.label(
                                device
                                    .rssi
                                    .map_or("--".to_string(), |r| format!("{r} dBm")),
                            );
                            // Show shortened peripheral ID
                            let short_id = if device.peripheral_id.len() > 16 {
                                &device.peripheral_id[..16]
                            } else {
                                &device.peripheral_id
                            };
                            ui.label(short_id);
                            if state.connection.phase != ConnectionPhase::Connected
                                && state.connection.phase != ConnectionPhase::Connecting
                            {
                                if ui.button("Connect").clicked() {
                                    connect_idx = Some(idx);
                                }
                            }
                            ui.end_row();
                        }

                        if let Some(idx) = connect_idx {
                            let pin: u32 = state
                                .connection
                                .config_pin
                                .parse()
                                .unwrap_or(9999);
                            let device = &state.connection.scanned_devices[idx];
                            state.connection.selected_device_index = Some(idx);
                            state.connection.phase = ConnectionPhase::Connecting;
                            state.connection.error_message = None;
                            ble.send(BleCommand::Connect {
                                peripheral_id: device.peripheral_id.clone(),
                                config_pin: pin,
                            });
                        }
                    });
            });
    }

    // Error display
    if let Some(err) = &state.connection.error_message {
        ui.add_space(8.0);
        ui.colored_label(egui::Color32::from_rgb(255, 80, 80), format!("Error: {err}"));
    }
}
