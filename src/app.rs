use eframe::egui;
use uuid::Uuid;
use log::{info, debug, warn};

use crate::state::*;
use crate::ble::events::BleEvent;
use crate::ble::manager::BleHandle;
use crate::protocol::{uuids, codec, calibration};
use crate::protocol::types::{StatusByte, DataUnits, LinearisationEntry};
use crate::ui;

pub struct B24App {
    state: AppState,
    ble: BleHandle,
}

impl B24App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Increase default font sizes globally
        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
        style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(15.0));
        style.text_styles.insert(egui::TextStyle::Monospace, egui::FontId::monospace(15.0));
        style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(22.0));
        style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
        // Bigger spacing for inputs
        style.spacing.interact_size.y = 30.0;
        style.spacing.text_edit_width = 200.0;
        cc.egui_ctx.set_style(style);

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
                    manufacturer_data,
                    service_uuids,
                } => {
                    // Determine if this is a B24 device:
                    // Check for Mantracourt manufacturer ID (0x04C3) or B24 config service UUID
                    let b24_svc = uuids::svc_config();
                    let is_b24 = service_uuids.contains(&b24_svc)
                        || manufacturer_data.contains_key(&0x04C3);

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
                        if is_b24 {
                            dev.is_b24 = true;
                        }
                        dev.manufacturer_data = manufacturer_data;
                    } else {
                        self.state.connection.scanned_devices.push(ScannedDevice {
                            name: name.unwrap_or_else(|| "Unknown".to_string()),
                            data_tag: None,
                            rssi,
                            peripheral_id: peripheral_id.clone(),
                            manufacturer_data,
                            service_uuids,
                            is_b24,
                            decoded_tag: None,
                            decoded_value: None,
                            decoded_units: None,
                            decoded_status: None,
                            decoded_pin_valid: false,
                        });
                    }

                    // Auto-decode advertising data for device cards (using default View PIN)
                    if let Some(dev) = self.state.connection.scanned_devices
                        .iter_mut()
                        .find(|d| d.peripheral_id == peripheral_id)
                    {
                        if let Some(raw) = dev.manufacturer_data.get(&0x04C3).cloned() {
                            // Decode with empty PIN (default is null bytes)
                            if let Some(decoded) = codec::decode_advertising(&raw, "") {
                                dev.decoded_tag = Some(decoded.data_tag);
                                dev.decoded_pin_valid = decoded.pin_valid;
                                if decoded.pin_valid {
                                    dev.decoded_value = Some(decoded.value);
                                    dev.decoded_units = Some(decoded.units);
                                    dev.decoded_status = Some(decoded.status);
                                }
                            }
                        }
                    }

                    // If view mode is active (advertising), decode with user's View PIN
                    if self.state.ui.view_mode.active
                        && self.state.ui.view_mode.source == ViewSource::Advertising
                    {
                        if let Some(ref view_pid) = self.state.ui.view_mode.peripheral_id {
                            if *view_pid == peripheral_id {
                                if let Some(dev) = self.state.connection.scanned_devices
                                    .iter()
                                    .find(|d| d.peripheral_id == peripheral_id)
                                {
                                    if let Some(raw) = dev.manufacturer_data.get(&0x04C3) {
                                        if let Some(decoded) = codec::decode_advertising(
                                            raw,
                                            &self.state.ui.view_mode.view_pin,
                                        ) {
                                            if decoded.pin_valid {
                                                self.state.ui.view_mode.current_value = Some(decoded.value);
                                                self.state.ui.view_mode.current_units = Some(decoded.units);
                                                self.state.ui.view_mode.current_status =
                                                    Some(StatusByte::from_byte(decoded.status));
                                            }
                                            self.state.ui.view_mode.data_tag = Some(decoded.data_tag);
                                            self.state.ui.view_mode.last_update = Some(std::time::Instant::now());

                                            // Push to history only if PIN is valid
                                            if decoded.pin_valid {
                                                let start = self.state.ui.view_mode.start_time
                                                    .get_or_insert_with(std::time::Instant::now);
                                                let elapsed = start.elapsed().as_secs_f64();
                                                self.state.ui.view_mode.history.push_back((elapsed, decoded.value));
                                                if self.state.ui.view_mode.history.len() > 2000 {
                                                    self.state.ui.view_mode.history.pop_front();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                BleEvent::ManufacturerDataUpdate { peripheral_id, manufacturer_data } => {
                    // Fast path: update manufacturer data on existing device
                    if let Some(dev) = self.state.connection.scanned_devices
                        .iter_mut()
                        .find(|d| d.peripheral_id == peripheral_id)
                    {
                        dev.manufacturer_data = manufacturer_data;
                        // Re-decode for device card
                        if let Some(raw) = dev.manufacturer_data.get(&0x04C3).cloned() {
                            if let Some(decoded) = codec::decode_advertising(&raw, "") {
                                dev.decoded_tag = Some(decoded.data_tag);
                                dev.decoded_pin_valid = decoded.pin_valid;
                                if decoded.pin_valid {
                                    dev.decoded_value = Some(decoded.value);
                                    dev.decoded_units = Some(decoded.units);
                                    dev.decoded_status = Some(decoded.status);
                                }
                            }
                        }
                    }

                    // Fast path for view mode: decode directly from manufacturer data
                    if self.state.ui.view_mode.active
                        && self.state.ui.view_mode.source == ViewSource::Advertising
                    {
                        if let Some(ref view_pid) = self.state.ui.view_mode.peripheral_id {
                            if *view_pid == peripheral_id {
                                if let Some(dev) = self.state.connection.scanned_devices
                                    .iter()
                                    .find(|d| d.peripheral_id == peripheral_id)
                                {
                                    if let Some(raw) = dev.manufacturer_data.get(&0x04C3) {
                                        if let Some(decoded) = codec::decode_advertising(
                                            raw,
                                            &self.state.ui.view_mode.view_pin,
                                        ) {
                                            if decoded.pin_valid {
                                                self.state.ui.view_mode.current_value = Some(decoded.value);
                                                self.state.ui.view_mode.current_units = Some(decoded.units);
                                                self.state.ui.view_mode.current_status =
                                                    Some(StatusByte::from_byte(decoded.status));
                                            }
                                            self.state.ui.view_mode.data_tag = Some(decoded.data_tag);
                                            self.state.ui.view_mode.last_update = Some(std::time::Instant::now());

                                            if decoded.pin_valid {
                                                let start = self.state.ui.view_mode.start_time
                                                    .get_or_insert_with(std::time::Instant::now);
                                                let elapsed = start.elapsed().as_secs_f64();
                                                self.state.ui.view_mode.history.push_back((elapsed, decoded.value));
                                                if self.state.ui.view_mode.history.len() > 2000 {
                                                    self.state.ui.view_mode.history.pop_front();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
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
                    // Auto-switch to Configuration tab
                    self.state.ui.active_tab = Tab::Configuration;

                    // Auto-read all config + calibration registers (with tracking)
                    use crate::ble::commands::BleCommand;

                    let cmd1 = BleCommand::ReadAll(uuids::all_config_uuids());
                    self.state.track_send(&cmd1);
                    self.ble.send(cmd1);

                    let cmd2 = BleCommand::ReadAll(uuids::all_calibration_uuids());
                    self.state.track_send(&cmd2);
                    self.ble.send(cmd2);

                    let cmd3 = BleCommand::ReadCharacteristic(uuids::char_data_units());
                    self.state.track_send(&cmd3);
                    self.ble.send(cmd3);

                    // Auto-read all advanced params
                    for param in crate::protocol::types::AdvancedParam::ALL {
                        let cmd = BleCommand::ReadAdvanced { index: param.index() };
                        self.state.track_send(&cmd);
                        self.ble.send(cmd);
                    }
                }
                BleEvent::Disconnected { reason } => {
                    self.state.connection.phase = ConnectionPhase::Disconnected;
                    self.state.connection.show_pin_dialog = false;
                    self.state.connection.pending_connect_id = None;
                    if let Some(r) = reason {
                        self.state.connection.error_message = Some(r);
                    }
                    self.state.clear_pending();
                    // Switch back to Connect tab so user can reconnect
                    self.state.ui.active_tab = Tab::Connect;
                    info!("Disconnected");
                }
                BleEvent::CharacteristicRead { uuid, data } => {
                    self.state.ui.pending_reads.remove(&uuid);
                    self.apply_characteristic_read(uuid, &data);
                }
                BleEvent::CharacteristicWritten { uuid } => {
                    self.state.ui.pending_writes.remove(&uuid);
                    debug!("Written: {uuid}");
                }
                BleEvent::Notification { uuid, data } => {
                    self.apply_notification(uuid, &data);
                }
                BleEvent::AdvancedRead { index, data } => {
                    self.state.ui.pending_adv_reads.remove(&index);
                    self.state.advanced.values.insert(index, data);
                }
                BleEvent::AdvancedWritten { index } => {
                    self.state.ui.pending_adv_writes.remove(&index);
                    debug!("Advanced written: {index}");
                }
                BleEvent::ActionExecuted(action) => {
                    info!("Action executed: {:?}", action);
                }
                BleEvent::Error(e) => {
                    let msg = e.to_string();
                    warn!("BLE error: {msg}");
                    self.state.connection.error_message = Some(msg);
                    // If we were trying to connect or thought we were connected,
                    // reset fully back to Disconnected so user can retry
                    if self.state.connection.phase == ConnectionPhase::Connecting
                        || self.state.connection.phase == ConnectionPhase::Connected
                    {
                        self.state.connection.phase = ConnectionPhase::Disconnected;
                        self.state.connection.show_pin_dialog = false;
                        self.state.connection.pending_connect_id = None;
                        // Switch back to Connect tab
                        self.state.ui.active_tab = Tab::Connect;
                    }
                    // Clear all pending to prevent stuck spinners
                    self.state.clear_pending();
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
            // View PIN is a 4-character ASCII string (e.g., "1234" or "0000")
            self.state.config.view_pin = Some(codec::decode_string(data));
        } else if uuid == uuids::char_serial_number() {
            // Serial number is Uint32
            self.state.config.serial_number = codec::decode_u32_be(data).ok();
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
            // Firmware version is stored as a float (e.g., 3.01)
            self.state.config.firmware_version = codec::decode_f32_be(data).ok();
        }
        // Calibration characteristics
        else if uuid == uuids::char_sens_range() {
            self.state.calibration.sensitivity_range = codec::decode_u8(data).ok();
        } else if uuid == uuids::char_coeff_at_idx() {
            self.state.calibration.coefficient = codec::decode_f32_be(data).ok();
            // Process linearisation table read queue
            if !self.state.calibration.lin_read_queue.is_empty() {
                if let Some(val) = self.state.calibration.coefficient {
                    self.state.calibration.lin_read_buf.push(val);
                    self.state.calibration.lin_read_queue.remove(0);
                    if let Some((next_idx, _)) = self.state.calibration.lin_read_queue.first() {
                        // Read next coefficient
                        let next_idx = *next_idx;
                        self.ble.send(crate::ble::commands::BleCommand::WriteCharacteristic {
                            uuid: uuids::char_lin_index(),
                            data: codec::encode_u8(next_idx),
                        });
                        self.ble.send(crate::ble::commands::BleCommand::ReadCharacteristic(
                            uuids::char_coeff_at_idx(),
                        ));
                    } else {
                        // All done — build the table
                        let buf = &self.state.calibration.lin_read_buf;
                        let np = self.state.calibration.lin_read_num_points as usize;
                        let mut table = Vec::new();
                        // Each segment: 3 values (valid_from, gain, offset) + final valid_to
                        for i in 0..np {
                            let base = i * 3;
                            if base + 2 < buf.len() {
                                let valid_to = if base + 3 < buf.len() {
                                    buf[base + 3]
                                } else {
                                    0.0
                                };
                                table.push(LinearisationEntry {
                                    index: (i + 1) as u8,
                                    valid_from: buf[base],
                                    gain: buf[base + 1],
                                    offset: buf[base + 2],
                                    valid_to,
                                });
                            }
                        }
                        self.state.calibration.linearisation_table = table;
                        self.state.calibration.lin_read_buf.clear();
                    }
                }
            }
        } else if uuid == uuids::char_lin_index() {
            self.state.calibration.lin_index = codec::decode_u8(data).ok();
        } else if uuid == uuids::char_lin_repeat() {
            self.state.calibration.lin_repeat = codec::decode_u8(data).ok();
        } else if uuid == uuids::char_lin_points() {
            self.state.calibration.lin_points = codec::decode_u8(data).ok();
            // Auto-trigger linearisation table read
            if let Some(num_points) = self.state.calibration.lin_points {
                if num_points > 0 && num_points <= 15 {
                    let seq = calibration::linearisation_read_sequence(num_points);
                    self.state.calibration.lin_read_queue = seq.iter()
                        .map(|(idx, name)| (*idx, name.to_string()))
                        .collect();
                    self.state.calibration.lin_read_buf.clear();
                    self.state.calibration.lin_read_num_points = num_points;
                    // Start reading the first coefficient
                    if let Some((idx, _)) = self.state.calibration.lin_read_queue.first() {
                        let idx = *idx;
                        self.ble.send(crate::ble::commands::BleCommand::WriteCharacteristic {
                            uuid: uuids::char_lin_index(),
                            data: codec::encode_u8(idx),
                        });
                        self.ble.send(crate::ble::commands::BleCommand::ReadCharacteristic(
                            uuids::char_coeff_at_idx(),
                        ));
                    }
                }
            }
        } else if uuid == uuids::char_base_value() {
            self.state.calibration.base_value = codec::decode_f32_be(data).ok();
            // If auto_cal is waiting for a capture, feed the value to the correct point
            if let Some(idx) = self.state.ui.auto_cal.capturing_index {
                if let Some(base) = self.state.calibration.base_value {
                    if let Some(point) = self.state.ui.auto_cal.points.get_mut(idx) {
                        point.base_value = Some(base);
                        point.capture_status = CaptureStatus::Captured;
                    }
                }
                self.state.ui.auto_cal.capturing_index = None;
            }
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

        // Request repaint when connected, scanning, connecting, view mode, or pending BLE operations
        if self.state.connection.phase == ConnectionPhase::Connected
            || self.state.connection.phase == ConnectionPhase::Scanning
            || self.state.connection.phase == ConnectionPhase::Connecting
            || self.state.has_pending()
            || self.state.ui.view_mode.active
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        // Top panel: tab bar + disconnect button
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
                ui.selectable_value(&mut self.state.ui.active_tab, Tab::MobileExport, "Mobile Export");

                // Right-aligned disconnect button (visible when connected/connecting, but NOT on Connect tab)
                let phase = self.state.connection.phase;
                let on_connect_tab = self.state.ui.active_tab == Tab::Connect;
                if !on_connect_tab && (phase == ConnectionPhase::Connected || phase == ConnectionPhase::Connecting) {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(
                            egui::Button::new(
                                egui::RichText::new("Disconnect").color(egui::Color32::WHITE)
                            ).fill(ui::widgets::COLOR_BTN_RED)
                        ).clicked() {
                            self.ble.send(crate::ble::commands::BleCommand::Disconnect);
                            self.state.connection.phase = ConnectionPhase::Disconnected;
                            self.state.connection.show_pin_dialog = false;
                            self.state.connection.pending_connect_id = None;
                            self.state.clear_pending();
                            self.state.ui.active_tab = Tab::Connect;
                        }
                        if phase == ConnectionPhase::Connecting {
                            ui.spinner();
                        }
                    });
                }
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
                    ui::calibration_tab::show(ui, &mut self.state, &self.ble);
                }
                Tab::Live => ui::live_tab::show(ui, &mut self.state, &self.ble),
                Tab::Log => ui::log_tab::show(ui, &mut self.state, &self.ble),
                Tab::MobileExport => ui::mobile_export_tab::show(ui, &mut self.state, &self.ble),
            }
        });
    }
}
