use eframe::egui;
use egui_plot::{Plot, Line, PlotPoints};
use crate::state::{AppState, ConnectionPhase, ViewSource};
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::types::DataUnits;
use crate::ui::widgets::{self, status_indicator};

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    // If view mode is active, show the view screen instead of normal connect UI
    if state.ui.view_mode.active {
        show_view_mode(ui, state);
        return;
    }

    let is_scanning = state.connection.phase == ConnectionPhase::Scanning;
    let is_connected = state.connection.phase == ConnectionPhase::Connected;
    let is_connecting = state.connection.phase == ConnectionPhase::Connecting;

    ui.add_space(8.0);

    // Header row with scan/disconnect controls
    ui.horizontal(|ui| {
        widgets::page_header(ui, "B24 Devices");
        ui.add_space(16.0);

        if !is_connected && !is_connecting {
            let scan_btn = if is_scanning {
                egui::Button::new("Stop Scan")
            } else {
                egui::Button::new("Start Scan")
            };
            if ui.add_sized([120.0, widgets::BTN_HEIGHT_HEADER], scan_btn).clicked() {
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
                    [120.0, widgets::BTN_HEIGHT_HEADER],
                    egui::Button::new(
                        egui::RichText::new("Disconnect").color(egui::Color32::WHITE)
                    ).fill(widgets::COLOR_BTN_RED),
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
            ui.colored_label(widgets::COLOR_ERROR, format!("Error: {err}"));
        });
        ui.add_space(4.0);
    }

    // -- Config PIN Dialog --
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
                response.request_focus();

                ui.add_space(16.0);

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

    // -- View PIN Dialog --
    if state.ui.view_mode.show_pin_dialog {
        ui.separator();
        ui.add_space(8.0);

        let frame = egui::Frame::none()
            .inner_margin(16.0)
            .rounding(8.0)
            .fill(egui::Color32::from_rgb(35, 45, 40))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 180, 120)));

        frame.show(ui, |ui| {
            ui.heading("Enter View PIN");
            ui.add_space(8.0);
            ui.label("Enter the 4-character View PIN to decode broadcast data.");
            ui.label("This allows viewing sensor data without a full connection.");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("View PIN:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.ui.view_mode.view_pin)
                        .desired_width(120.0)
                        .hint_text("0000")
                        .font(egui::TextStyle::Monospace),
                );
                response.request_focus();

                ui.add_space(16.0);

                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.add_sized([100.0, widgets::BTN_HEIGHT_INLINE], egui::Button::new(
                    egui::RichText::new("View").color(egui::Color32::WHITE)
                ).fill(widgets::COLOR_BTN_GREEN)).clicked()
                    || enter_pressed
                {
                    // Activate view mode
                    state.ui.view_mode.show_pin_dialog = false;
                    state.ui.view_mode.active = true;
                    state.ui.view_mode.source = ViewSource::Advertising;
                    state.ui.view_mode.current_value = None;
                    state.ui.view_mode.current_units = None;
                    state.ui.view_mode.current_status = None;
                    state.ui.view_mode.data_tag = None;
                    state.ui.view_mode.history.clear();
                    state.ui.view_mode.start_time = None;

                    // Ensure scanning is active so we receive advertising packets
                    if state.connection.phase != ConnectionPhase::Scanning {
                        state.connection.scanned_devices.clear();
                        ble.send(BleCommand::StartScan);
                        state.connection.phase = ConnectionPhase::Scanning;
                    }
                }

                if ui.button("Cancel").clicked() {
                    state.ui.view_mode.show_pin_dialog = false;
                    state.ui.view_mode.peripheral_id = None;
                    state.ui.view_mode.device_name.clear();
                    state.ui.view_mode.view_pin.clear();
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
            let mut clicked_view_idx = None;

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
                            // Row 1: Name + Data Tag
                            let title = if let Some(tag) = device.decoded_tag {
                                format!("{} ({:04X})", device.name, tag)
                            } else {
                                device.name.clone()
                            };
                            ui.label(
                                egui::RichText::new(&title)
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.add_space(2.0);
                            // Row 2: RSSI | Value+Units (if decoded) | Peripheral ID
                            ui.horizontal(|ui| {
                                let rssi = device.rssi.unwrap_or(-100);
                                let signal_color = if rssi > -60 {
                                    widgets::COLOR_SUCCESS
                                } else if rssi > -80 {
                                    widgets::COLOR_WARNING
                                } else {
                                    widgets::COLOR_ERROR
                                };
                                ui.colored_label(signal_color, format!("{rssi} dBm"));

                                if device.decoded_pin_valid {
                                    if let Some(val) = device.decoded_value {
                                        ui.separator();
                                        let units_str = device.decoded_units
                                            .map(|u| DataUnits::from_byte(u).label().to_string())
                                            .unwrap_or_default();
                                        ui.label(
                                            egui::RichText::new(format!("{val:.6} {units_str}"))
                                                .color(egui::Color32::from_rgb(120, 200, 255)),
                                        );
                                    }
                                }

                                ui.separator();
                                ui.label(
                                    egui::RichText::new(device.peripheral_id.to_uppercase())
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(120, 120, 130)),
                                );
                            });
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !is_connected && !is_connecting
                                && !state.connection.show_pin_dialog
                                && !state.ui.view_mode.show_pin_dialog
                            {
                                if ui
                                    .add_sized([100.0, widgets::BTN_HEIGHT_HEADER], egui::Button::new(
                                        egui::RichText::new("Connect").color(egui::Color32::WHITE)
                                    ).fill(widgets::COLOR_BTN_GREEN))
                                    .clicked()
                                {
                                    clicked_connect_idx = Some(idx);
                                }
                                ui.add_space(4.0);
                                if ui
                                    .add_sized([80.0, widgets::BTN_HEIGHT_HEADER], egui::Button::new(
                                        egui::RichText::new("View")
                                    ))
                                    .clicked()
                                {
                                    clicked_view_idx = Some(idx);
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

            // When user clicks View on a device, show View PIN dialog
            if let Some(idx) = clicked_view_idx {
                state.connection.selected_device_index = Some(idx);
                let dev = &state.connection.scanned_devices[idx];
                state.ui.view_mode.peripheral_id = Some(dev.peripheral_id.clone());
                state.ui.view_mode.device_name = dev.name.clone();
                state.ui.view_mode.show_pin_dialog = true;
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

// -- View Mode Screen -------------------------------------------------------

pub fn show_view_mode(ui: &mut egui::Ui, state: &mut AppState) {
    // Determine data source
    let is_connected_view = state.ui.view_mode.source == ViewSource::Connected;

    // Get values from the appropriate source
    let (current_value, current_units, current_status) = if is_connected_view {
        (
            state.live_data.current_value,
            state.live_data.current_units,
            state.live_data.current_status,
        )
    } else {
        (
            state.ui.view_mode.current_value,
            state.ui.view_mode.current_units,
            state.ui.view_mode.current_status,
        )
    };

    // Header bar
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if is_connected_view {
            ui.label(
                egui::RichText::new("Live View (Connected)")
                    .size(18.0)
                    .strong()
                    .color(widgets::COLOR_SUCCESS),
            );
        } else {
            ui.label(
                egui::RichText::new(&state.ui.view_mode.device_name)
                    .size(18.0)
                    .strong(),
            );

            // Signal strength from the scanned device
            if let Some(ref pid) = state.ui.view_mode.peripheral_id {
                if let Some(dev) = state.connection.scanned_devices.iter().find(|d| d.peripheral_id == *pid) {
                    let rssi = dev.rssi.unwrap_or(-100);
                    let signal_color = if rssi > -60 {
                        widgets::COLOR_SUCCESS
                    } else if rssi > -80 {
                        widgets::COLOR_WARNING
                    } else {
                        widgets::COLOR_ERROR
                    };
                    ui.separator();
                    ui.colored_label(signal_color, format!("{rssi} dBm"));
                }
            }

            if let Some(tag) = state.ui.view_mode.data_tag {
                ui.separator();
                ui.label(format!("Tag: {tag:04X}"));
            }

            if let Some(last) = state.ui.view_mode.last_update {
                ui.separator();
                let ago = last.elapsed().as_secs_f32();
                let color = if ago < 3.0 {
                    widgets::COLOR_SUCCESS
                } else if ago < 10.0 {
                    widgets::COLOR_WARNING
                } else {
                    widgets::COLOR_ERROR
                };
                ui.colored_label(color, format!("{ago:.0}s ago"));
            }
        }

        // Right-aligned Back button
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_sized([100.0, 28.0], egui::Button::new(
                egui::RichText::new("<< Back").size(15.0)
            )).clicked() {
                exit_view_mode(state);
            }
        });
    });

    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // -- HUGE value display --
        ui.add_space(8.0);

        let value_text = current_value
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "--".to_string());

        let units_text = current_units
            .map(|u| DataUnits::from_byte(u).label().to_string())
            .unwrap_or_default();

        // Measure text sizes to center value+units as a group
        let value_galley = ui.painter().layout_no_wrap(
            value_text.clone(),
            egui::FontId::monospace(140.0),
            egui::Color32::WHITE,
        );
        let units_galley = ui.painter().layout_no_wrap(
            units_text.clone(),
            egui::FontId::proportional(28.0),
            egui::Color32::from_rgb(160, 200, 255),
        );
        let total_w = value_galley.size().x + 12.0 + units_galley.size().x;
        let value_h = value_galley.size().y;
        let units_h = units_galley.size().y;
        let row_h = value_h;
        let avail_w = ui.available_width();
        let start_x = (avail_w - total_w) / 2.0;

        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(avail_w, row_h),
            egui::Sense::hover(),
        );
        // Value: vertically centered
        let value_pos = rect.min + egui::vec2(start_x, 0.0);
        ui.painter().galley(value_pos, value_galley, egui::Color32::WHITE);
        // Units: bottom-aligned with value
        let units_pos = rect.min + egui::vec2(
            start_x + (total_w - units_galley.size().x),
            row_h - units_h - 4.0,
        );
        ui.painter().galley(units_pos, units_galley, egui::Color32::from_rgb(160, 200, 255));

        ui.add_space(4.0);

        // -- Status flags --
        if let Some(status) = current_status {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(16.0);
                status_indicator(ui, "ShuntCal", status.shunt_cal);
                status_indicator(ui, "Integrity", status.integrity);
                status_indicator(ui, "Tare", status.not_gross);
                status_indicator(ui, "OverRange", status.over_range);
                status_indicator(ui, "Fast", status.fast_mode);
                status_indicator(ui, "BattLow", status.battery_low);
                status_indicator(ui, "DigIn", status.digital_input);
                ui.separator();
                ui.label(status.description());
            });
        }

        ui.add_space(4.0);

        // -- Time-series chart --
        let history: Vec<[f64; 2]> = if is_connected_view {
            state.live_data.history.iter().map(|(t, v)| [*t, *v as f64]).collect()
        } else {
            state.ui.view_mode.history.iter().map(|(t, v)| [*t, *v as f64]).collect()
        };

        let points: PlotPoints = history.into_iter().collect();

        let plot = Plot::new("view_plot")
            .view_aspect(3.0)
            .x_axis_label("Time (s)")
            .y_axis_label("Value")
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(true);

        plot.show(ui, |plot_ui| {
            plot_ui.line(
                Line::new(points)
                    .name("Data Value")
                    .color(egui::Color32::from_rgb(50, 200, 120)),
            );
        });

        ui.add_space(8.0);

        // -- Stats --
        let history_values: Vec<f32> = if is_connected_view {
            state.live_data.history.iter().map(|(_, v)| *v).collect()
        } else {
            state.ui.view_mode.history.iter().map(|(_, v)| *v).collect()
        };

        if !history_values.is_empty() {
            let min = history_values.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = history_values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let avg: f32 = history_values.iter().sum::<f32>() / history_values.len() as f32;

            ui.horizontal(|ui| {
                ui.label(format!("Points: {}", history_values.len()));
                ui.separator();
                ui.label(format!("Min: {min:.6}"));
                ui.separator();
                ui.label(format!("Max: {max:.6}"));
                ui.separator();
                ui.label(format!("Avg: {avg:.6}"));

                ui.add_space(16.0);

                if ui.button("Clear History").clicked() {
                    if is_connected_view {
                        state.live_data.history.clear();
                        state.live_data.start_time = None;
                    } else {
                        state.ui.view_mode.history.clear();
                        state.ui.view_mode.start_time = None;
                    }
                }
            });
        } else if !is_connected_view {
            ui.label(
                egui::RichText::new("Waiting for advertising data...")
                    .size(14.0)
                    .color(egui::Color32::from_rgb(140, 140, 140)),
            );
        }

        if !is_connected_view {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Update rate depends on the device's Data Rate setting (connect to change)")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(100, 100, 110)),
            );
        }
    });
}

fn exit_view_mode(state: &mut AppState) {
    state.ui.view_mode.active = false;
    state.ui.view_mode.peripheral_id = None;
    state.ui.view_mode.device_name.clear();
    state.ui.view_mode.current_value = None;
    state.ui.view_mode.current_units = None;
    state.ui.view_mode.current_status = None;
    state.ui.view_mode.data_tag = None;
    state.ui.view_mode.history.clear();
    state.ui.view_mode.start_time = None;
    state.ui.view_mode.last_update = None;
    state.ui.view_mode.source = ViewSource::Advertising;
}
