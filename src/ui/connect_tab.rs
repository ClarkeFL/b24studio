use eframe::egui;
use crate::state::{AppState, ConnectionPhase};
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    let is_scanning = state.connection.phase == ConnectionPhase::Scanning;
    let is_connected = state.connection.phase == ConnectionPhase::Connected;
    let is_connecting = state.connection.phase == ConnectionPhase::Connecting;

    ui.add_space(8.0);

    // Header row with scan/disconnect controls
    ui.horizontal(|ui| {
        ui.heading("B24 Devices");
        ui.add_space(16.0);

        if !is_connected && !is_connecting {
            let scan_btn = if is_scanning {
                egui::Button::new("Stop Scan")
            } else {
                egui::Button::new("Start Scan")
            };
            if ui.add_sized([120.0, 28.0], scan_btn).clicked() {
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

        if is_connected || is_connecting {
            if ui
                .add_sized(
                    [120.0, 28.0],
                    egui::Button::new("Disconnect")
                        .fill(egui::Color32::from_rgb(180, 50, 50)),
                )
                .clicked()
            {
                ble.send(BleCommand::Disconnect);
                state.connection.phase = ConnectionPhase::Disconnected;
                state.connection.show_pin_dialog = false;
                state.connection.pending_connect_id = None;
                state.clear_pending();
            }
        }

        if is_connecting {
            ui.spinner();
            ui.label("Connecting...");
        }
    });

    ui.add_space(8.0);

    // Error display
    if let Some(err) = &state.connection.error_message {
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(255, 80, 80), format!("Error: {err}"));
        });
        ui.add_space(4.0);
    }

    // ── PIN Dialog ──────────────────────────────────────────────
    if state.connection.show_pin_dialog {
        ui.separator();
        ui.add_space(8.0);

        let frame = egui::Frame::none()
            .inner_margin(16.0)
            .rounding(8.0)
            .fill(egui::Color32::from_rgb(35, 40, 55))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 140, 220)));

        frame.show(ui, |ui| {
            ui.heading("Enter Configuration PIN");
            ui.add_space(8.0);
            ui.label("The B24 module requires a Configuration PIN to allow access.");
            ui.label("Default PIN is 0 (zero). Enter the PIN and click Connect.");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("PIN:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.connection.config_pin)
                        .desired_width(120.0)
                        .hint_text("0")
                        .font(egui::TextStyle::Monospace),
                );
                // Auto-focus the PIN field
                response.request_focus();

                ui.add_space(16.0);

                // Submit on Enter key (while field is focused) or button click
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.add_sized([100.0, 28.0], egui::Button::new("Connect")).clicked()
                    || enter_pressed
                {
                    do_connect(state, ble);
                }

                if ui.button("Cancel").clicked() {
                    state.connection.show_pin_dialog = false;
                    state.connection.pending_connect_id = None;
                }
            });
        });

        ui.add_space(8.0);
    }

    ui.separator();
    ui.add_space(4.0);

    // Filter: only show B24 devices
    let b24_devices: Vec<usize> = state
        .connection
        .scanned_devices
        .iter()
        .enumerate()
        .filter(|(_, d)| d.is_b24)
        .map(|(i, _)| i)
        .collect();

    if b24_devices.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            if is_scanning {
                ui.spinner();
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Scanning for B24 devices...")
                        .size(16.0)
                        .color(egui::Color32::from_rgb(180, 180, 180)),
                );
                let total = state.connection.scanned_devices.len();
                if total > 0 {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Found {total} BLE device(s), none identified as B24 yet"
                        ))
                        .size(12.0)
                        .color(egui::Color32::from_rgb(140, 140, 140)),
                    );
                }
            } else {
                ui.label(
                    egui::RichText::new("No B24 devices found")
                        .size(16.0)
                        .color(egui::Color32::from_rgb(180, 180, 180)),
                );
                ui.add_space(8.0);
                ui.label("Click 'Start Scan' to discover nearby B24 modules.");
            }
        });
    } else {
        // Device cards
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut clicked_connect_idx = None;

            for &idx in &b24_devices {
                let device = &state.connection.scanned_devices[idx];
                let is_selected = state.connection.selected_device_index == Some(idx);

                let frame = egui::Frame::none()
                    .inner_margin(12.0)
                    .rounding(6.0)
                    .fill(if is_selected {
                        egui::Color32::from_rgb(40, 55, 80)
                    } else {
                        egui::Color32::from_rgb(35, 35, 40)
                    })
                    .stroke(egui::Stroke::new(
                        1.0,
                        if is_selected {
                            egui::Color32::from_rgb(80, 140, 220)
                        } else {
                            egui::Color32::from_rgb(60, 60, 70)
                        },
                    ));

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(&device.name)
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                let rssi = device.rssi.unwrap_or(-100);
                                let signal_color = if rssi > -60 {
                                    egui::Color32::from_rgb(80, 200, 80)
                                } else if rssi > -80 {
                                    egui::Color32::from_rgb(255, 200, 50)
                                } else {
                                    egui::Color32::from_rgb(255, 80, 80)
                                };
                                ui.colored_label(signal_color, format!("{rssi} dBm"));
                                ui.separator();
                                let short_id = if device.peripheral_id.len() > 20 {
                                    &device.peripheral_id[..20]
                                } else {
                                    &device.peripheral_id
                                };
                                ui.label(
                                    egui::RichText::new(short_id)
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(140, 140, 140)),
                                );
                            });
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !is_connected && !is_connecting && !state.connection.show_pin_dialog {
                                if ui
                                    .add_sized([100.0, 32.0], egui::Button::new("Connect"))
                                    .clicked()
                                {
                                    clicked_connect_idx = Some(idx);
                                }
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }

            // When user clicks Connect on a device, show PIN dialog
            if let Some(idx) = clicked_connect_idx {
                state.connection.selected_device_index = Some(idx);
                let pid = state.connection.scanned_devices[idx].peripheral_id.clone();
                state.connection.pending_connect_id = Some(pid);
                state.connection.show_pin_dialog = true;
                state.connection.error_message = None;
            }
        });
    }
}

fn do_connect(state: &mut AppState, ble: &BleHandle) {
    state.connection.show_pin_dialog = false;
    if let Some(pid) = state.connection.pending_connect_id.take() {
        let pin: u32 = state.connection.config_pin.parse().unwrap_or(0);
        state.connection.phase = ConnectionPhase::Connecting;
        state.connection.error_message = None;
        ble.send(BleCommand::Connect {
            peripheral_id: pid,
            config_pin: pin,
        });
    }
}
