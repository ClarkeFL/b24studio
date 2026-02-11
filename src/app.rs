use eframe::egui;
use uuid::Uuid;
use log::{info, debug, warn};

use crate::state::*;
use crate::ble::events::BleEvent;
use crate::ble::manager::BleHandle;
use crate::protocol::{uuids, codec};
use crate::protocol::types::{StatusByte, DataUnits};
use crate::ui;

pub struct B24App {
    state: AppState,
    ble: BleHandle,
}

impl B24App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let ble = crate::ble::manager::spawn_ble_worker();
        Self {
            state: AppState::default(),
            ble,
        }
    }

    fn process_ble_events(&mut self) {
        while let Ok(event) = self.ble.evt_rx.try_recv() {
            match event {
                BleEvent::DeviceDiscovered {
                    peripheral_id,
                    name,
                    rssi,
                } => {
                    // Deduplicate by peripheral_id
                    let existing = self
                        .state
                        .connection
                        .scanned_devices
                        .iter_mut()
                        .find(|d| d.peripheral_id == peripheral_id);
                    if let Some(dev) = existing {
                        if let Some(n) = &name {
                            dev.name = n.clone();
                        }
                        dev.rssi = rssi;
                    } else {
                        self.state.connection.scanned_devices.push(ScannedDevice {
                            name: name.unwrap_or_else(|| "Unknown".to_string()),
                            data_tag: None,
                            rssi,
                            peripheral_id,
                        });
                    }
                }
                BleEvent::ScanStopped => {
                    if self.state.connection.phase == ConnectionPhase::Scanning {
                        self.state.connection.phase = ConnectionPhase::Disconnected;
                    }
                }
                BleEvent::Connected => {
                    self.state.connection.phase = ConnectionPhase::Connected;
                    self.state.connection.error_message = None;
                    info!("Connected to device");
                }
                BleEvent::Disconnected { reason } => {
                    self.state.connection.phase = ConnectionPhase::Disconnected;
                    if let Some(r) = reason {
                        self.state.connection.error_message = Some(r);
                    }
                    info!("Disconnected");
                }
                BleEvent::CharacteristicRead { uuid, data } => {
                    self.apply_characteristic_read(uuid, &data);
                }
                BleEvent::CharacteristicWritten { uuid } => {
                    debug!("Written: {uuid}");
                }
                BleEvent::Notification { uuid, data } => {
                    self.apply_notification(uuid, &data);
                }
                BleEvent::AdvancedRead { index, data } => {
                    self.state.advanced.values.insert(index, data);
                }
                BleEvent::AdvancedWritten { index } => {
                    debug!("Advanced written: {index}");
                }
                BleEvent::ActionExecuted(action) => {
                    info!("Action executed: {:?}", action);
                }
                BleEvent::Error(e) => {
                    let msg = e.to_string();
                    warn!("BLE error: {msg}");
                    self.state.connection.error_message = Some(msg);
                }
            }
        }
    }

    fn apply_characteristic_read(&mut self, uuid: Uuid, data: &[u8]) {
        // Configuration characteristics
        if uuid == uuids::char_config_pin() {
            self.state.config.config_pin = codec::decode_u32_be(data).ok();
        } else if uuid == uuids::char_data_rate() {
            self.state.config.data_rate = codec::decode_u32_be(data).ok();
        } else if uuid == uuids::char_resolution() {
            self.state.config.resolution = codec::decode_u8(data).ok();
        } else if uuid == uuids::char_battery_thresh() {
            self.state.config.battery_threshold = codec::decode_f32_be(data).ok();
        } else if uuid == uuids::char_view_pin() {
            self.state.config.view_pin = Some(codec::decode_string(data));
        } else if uuid == uuids::char_serial_number() {
            self.state.config.serial_number = Some(codec::decode_string(data));
        } else if uuid == uuids::char_data_tag() {
            // Data tag is u16, display as hex
            if let Ok(tag) = codec::decode_u16_be(data) {
                self.state.config.data_tag = Some(format!("{tag:04X}"));
            }
        } else if uuid == uuids::char_battery_value() {
            self.state.config.battery_value = codec::decode_f32_be(data).ok();
        } else if uuid == uuids::char_system_zero() {
            self.state.config.system_zero = codec::decode_f32_be(data).ok();
        } else if uuid == uuids::char_model_name() {
            self.state.config.model_name = Some(codec::decode_string(data));
        } else if uuid == uuids::char_firmware_ver() {
            self.state.config.firmware_version = Some(codec::decode_string(data));
        }
        // Calibration characteristics
        else if uuid == uuids::char_sens_range() {
            self.state.calibration.sensitivity_range = codec::decode_u8(data).ok();
        } else if uuid == uuids::char_coeff_at_idx() {
            self.state.calibration.coefficient = codec::decode_f32_be(data).ok();
        } else if uuid == uuids::char_lin_index() {
            self.state.calibration.lin_index = codec::decode_u8(data).ok();
        } else if uuid == uuids::char_lin_repeat() {
            self.state.calibration.lin_repeat = codec::decode_u8(data).ok();
        } else if uuid == uuids::char_lin_points() {
            self.state.calibration.lin_points = codec::decode_u8(data).ok();
        } else if uuid == uuids::char_base_value() {
            self.state.calibration.base_value = codec::decode_f32_be(data).ok();
        } else if uuid == uuids::char_base_units() {
            self.state.calibration.base_units = codec::decode_u8(data).ok();
        } else if uuid == uuids::char_data_gain() {
            self.state.calibration.data_gain = codec::decode_f32_be(data).ok();
        } else if uuid == uuids::char_data_offset() {
            self.state.calibration.data_offset = codec::decode_f32_be(data).ok();
        } else if uuid == uuids::char_cal_pin() {
            self.state.calibration.cal_pin = codec::decode_u32_be(data).ok();
        } else if uuid == uuids::char_cal_units() {
            self.state.calibration.cal_units = codec::decode_u8(data).ok();
        }
        // Data characteristics
        else if uuid == uuids::char_status() {
            if let Ok(b) = codec::decode_u8(data) {
                self.state.live_data.current_status = Some(StatusByte::from_byte(b));
            }
        } else if uuid == uuids::char_data_value() {
            if let Ok(v) = codec::decode_f32_be(data) {
                self.state.live_data.current_value = Some(v);
            }
        } else if uuid == uuids::char_data_units() {
            self.state.live_data.current_units = codec::decode_u8(data).ok();
        }
    }

    fn apply_notification(&mut self, uuid: Uuid, data: &[u8]) {
        if uuid == uuids::char_data_value() {
            if let Ok(val) = codec::decode_f32_be(data) {
                self.state.live_data.current_value = Some(val);

                // Initialize start time on first data point
                let start = self
                    .state
                    .live_data
                    .start_time
                    .get_or_insert_with(std::time::Instant::now);
                let elapsed = start.elapsed().as_secs_f64();

                self.state.live_data.history.push_back((elapsed, val));
                if self.state.live_data.history.len() > self.state.live_data.history_max_points {
                    self.state.live_data.history.pop_front();
                }

                // Log if logging
                if self.state.log.is_logging {
                    let units_label = self
                        .state
                        .live_data
                        .current_units
                        .map(|u| DataUnits::from_byte(u).label().to_string())
                        .unwrap_or_default();
                    let status = self
                        .state
                        .live_data
                        .current_status
                        .map(|s| s.to_byte())
                        .unwrap_or(0);
                    self.state.log.entries.push(LogEntry {
                        timestamp: chrono::Local::now(),
                        value: val,
                        status,
                        units: units_label,
                    });
                }
            }
        } else if uuid == uuids::char_status() {
            if let Some(&b) = data.first() {
                self.state.live_data.current_status = Some(StatusByte::from_byte(b));
            }
        } else if uuid == uuids::char_data_units() {
            if let Some(&b) = data.first() {
                self.state.live_data.current_units = Some(b);
            }
        }
    }
}

impl eframe::App for B24App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process BLE events
        self.process_ble_events();

        // Request repaint when connected (for live data updates)
        if self.state.connection.phase == ConnectionPhase::Connected {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // Top panel: tab bar
        egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.state.ui.active_tab, Tab::Connect, "Connect");
                ui.selectable_value(
                    &mut self.state.ui.active_tab,
                    Tab::Configuration,
                    "Configuration",
                );
                ui.selectable_value(
                    &mut self.state.ui.active_tab,
                    Tab::Calibration,
                    "Calibration",
                );
                ui.selectable_value(&mut self.state.ui.active_tab, Tab::Live, "Live Data");
                ui.selectable_value(&mut self.state.ui.active_tab, Tab::Log, "Log");
            });
        });

        // Bottom panel: status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui::status_bar::show(ui, &self.state);
        });

        // Central panel: active tab
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state.ui.active_tab {
                Tab::Connect => ui::connect_tab::show(ui, &mut self.state, &self.ble),
                Tab::Configuration => ui::config_tab::show(ui, &mut self.state, &self.ble),
                Tab::Calibration => {
                    // Show both calibration registers and wizard in same tab
                    ui.horizontal(|ui| {
                        ui.heading("Calibration");
                        ui.separator();
                        if ui.selectable_label(
                            self.state.ui.cal_sub_tab == CalSubTab::AutoCal
                                || self.state.ui.cal_sub_tab == CalSubTab::Multipoint,
                            "Wizard",
                        ).clicked() {
                            // Toggle between showing registers and wizard
                        }
                    });
                    ui.separator();

                    // Split: left = registers, right = wizard
                    let available = ui.available_width();
                    ui.horizontal(|ui| {
                        ui.allocate_ui(egui::vec2(available * 0.5, ui.available_height()), |ui| {
                            ui::calibration_tab::show(ui, &mut self.state, &self.ble);
                        });
                        ui.separator();
                        ui.allocate_ui(egui::vec2(available * 0.5, ui.available_height()), |ui| {
                            ui::cal_wizard_tab::show(ui, &mut self.state, &self.ble);
                        });
                    });
                }
                Tab::Live => ui::live_tab::show(ui, &mut self.state, &self.ble),
                Tab::Log => ui::log_tab::show(ui, &mut self.state, &self.ble),
            }
        });
    }
}
