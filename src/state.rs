use std::collections::{VecDeque, HashMap, HashSet};
use chrono::{DateTime, Local};
use uuid::Uuid;

use crate::ble::commands::BleCommand;
use crate::protocol::types::*;

/// Top-level application state. Owned exclusively by the UI thread.
#[derive(Default)]
pub struct AppState {
    pub connection: ConnectionState,
    pub config: ConfigRegisters,
    pub calibration: CalibrationRegisters,
    pub advanced: AdvancedRegisters,
    pub live_data: LiveData,
    pub log: DataLog,
    pub ui: UiState,
}

impl AppState {
    /// Track a BLE command as pending (for spinner display).
    pub fn track_send(&mut self, cmd: &BleCommand) {
        match cmd {
            BleCommand::ReadCharacteristic(uuid) => {
                self.ui.pending_reads.insert(*uuid);
            }
            BleCommand::WriteCharacteristic { uuid, .. } => {
                self.ui.pending_writes.insert(*uuid);
            }
            BleCommand::ReadAll(uuids) => {
                for u in uuids {
                    self.ui.pending_reads.insert(*u);
                }
            }
            BleCommand::ReadAdvanced { index } => {
                self.ui.pending_adv_reads.insert(*index);
            }
            BleCommand::WriteAdvanced { index, .. } => {
                self.ui.pending_adv_writes.insert(*index);
            }
            _ => {}
        }
    }

    /// Check if a characteristic UUID has a pending read or write.
    pub fn is_pending(&self, uuid: &Uuid) -> bool {
        self.ui.pending_reads.contains(uuid) || self.ui.pending_writes.contains(uuid)
    }

    /// Check if an advanced param index has a pending read or write.
    pub fn is_adv_pending(&self, index: u8) -> bool {
        self.ui.pending_adv_reads.contains(&index) || self.ui.pending_adv_writes.contains(&index)
    }

    /// Clear all pending operations (e.g. on disconnect or error).
    pub fn clear_pending(&mut self) {
        self.ui.pending_reads.clear();
        self.ui.pending_writes.clear();
        self.ui.pending_adv_reads.clear();
        self.ui.pending_adv_writes.clear();
    }

    /// Whether any BLE operations are pending.
    pub fn has_pending(&self) -> bool {
        !self.ui.pending_reads.is_empty()
            || !self.ui.pending_writes.is_empty()
            || !self.ui.pending_adv_reads.is_empty()
            || !self.ui.pending_adv_writes.is_empty()
    }
}

// ── Connection ─────────────────────────────────────────────────────

#[derive(Default)]
pub struct ConnectionState {
    pub phase: ConnectionPhase,
    pub scanned_devices: Vec<ScannedDevice>,
    pub config_pin: String,
    pub selected_device_index: Option<usize>,
    pub error_message: Option<String>,
    pub status_text: String,
    /// When true, shows PIN entry dialog before connecting
    pub show_pin_dialog: bool,
    /// Peripheral ID of the device we want to connect to (pending PIN entry)
    pub pending_connect_id: Option<String>,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum ConnectionPhase {
    #[default]
    Disconnected,
    Scanning,
    Connecting,
    Connected,
}

pub struct ScannedDevice {
    pub name: String,
    pub data_tag: Option<String>,
    pub rssi: Option<i16>,
    pub peripheral_id: String,
    pub manufacturer_data: std::collections::HashMap<u16, Vec<u8>>,
    pub service_uuids: Vec<uuid::Uuid>,
    pub is_b24: bool,
    // Decoded from advertising packet (auto-populated on each scan update)
    pub decoded_tag: Option<u16>,     // always correct (plaintext in packet)
    pub decoded_value: Option<f32>,   // only meaningful if decoded_pin_valid
    pub decoded_units: Option<u8>,    // only meaningful if decoded_pin_valid
    pub decoded_status: Option<u8>,   // only meaningful if decoded_pin_valid
    pub decoded_pin_valid: bool,      // true = default View PIN worked
}

// ── Configuration Registers ────────────────────────────────────────

#[derive(Default)]
pub struct ConfigRegisters {
    pub config_pin: Option<u32>,
    pub data_rate: Option<u32>,
    pub resolution: Option<u8>,
    pub battery_threshold: Option<f32>,
    pub view_pin: Option<String>,
    pub serial_number: Option<u32>,
    pub data_tag: Option<String>,
    pub battery_value: Option<f32>,
    pub system_zero: Option<f32>,
    pub model_name: Option<String>,
    pub firmware_version: Option<f32>,
}

// ── Calibration Registers ──────────────────────────────────────────

#[derive(Default)]
pub struct CalibrationRegisters {
    pub sensitivity_range: Option<u8>,
    pub coefficient: Option<f32>,
    pub lin_index: Option<u8>,
    pub lin_repeat: Option<u8>,
    pub lin_points: Option<u8>,
    pub base_value: Option<f32>,
    pub base_units: Option<u8>,
    pub data_gain: Option<f32>,
    pub data_offset: Option<f32>,
    pub cal_pin: Option<u32>,
    pub cal_units: Option<u8>,
    pub linearisation_table: Vec<LinearisationEntry>,
    /// Queue for reading linearisation table: (lin_index, field_name)
    pub lin_read_queue: Vec<(u8, String)>,
    /// Accumulated coefficients during a table read
    pub lin_read_buf: Vec<f32>,
    /// Number of points being read
    pub lin_read_num_points: u8,
}

// ── Advanced Registers ─────────────────────────────────────────────

#[derive(Default)]
pub struct AdvancedRegisters {
    pub values: HashMap<u8, Vec<u8>>,
}

impl AdvancedRegisters {
    pub fn get_f32(&self, index: u8) -> Option<f32> {
        self.values.get(&index).and_then(|d| {
            if d.len() >= 4 {
                Some(f32::from_be_bytes([d[0], d[1], d[2], d[3]]))
            } else {
                None
            }
        })
    }

    pub fn get_u32(&self, index: u8) -> Option<u32> {
        self.values.get(&index).and_then(|d| {
            if d.len() >= 4 {
                Some(u32::from_be_bytes([d[0], d[1], d[2], d[3]]))
            } else {
                None
            }
        })
    }

    pub fn get_u8(&self, index: u8) -> Option<u8> {
        self.values.get(&index).and_then(|d| d.first().copied())
    }
}

// ── Live Data ──────────────────────────────────────────────────────

pub struct LiveData {
    pub current_value: Option<f32>,
    pub current_status: Option<StatusByte>,
    pub current_units: Option<u8>,
    pub history: VecDeque<(f64, f32)>, // (elapsed_seconds, value)
    pub history_max_points: usize,
    pub start_time: Option<std::time::Instant>,
}

impl Default for LiveData {
    fn default() -> Self {
        Self {
            current_value: None,
            current_status: None,
            current_units: None,
            history: VecDeque::new(),
            history_max_points: 2000,
            start_time: None,
        }
    }
}

// ── Data Log ───────────────────────────────────────────────────────

#[derive(Default)]
pub struct DataLog {
    pub is_logging: bool,
    pub entries: Vec<LogEntry>,
}

pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub value: f32,
    pub status: u8,
    pub units: String,
}

// ── UI State ───────────────────────────────────────────────────────

pub struct UiState {
    pub active_tab: Tab,
    pub cal_sub_tab: CalSubTab,
    pub edit: EditBuffers,
    pub auto_cal: AutoCalState,
    pub table_cal: TableCalState,
    pub view_mode: ViewModeState,
    pub mobile_export: MobileExportState,
    pub pending_reads: HashSet<Uuid>,
    pub pending_writes: HashSet<Uuid>,
    pub pending_adv_reads: HashSet<u8>,
    pub pending_adv_writes: HashSet<u8>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active_tab: Tab::Connect,
            cal_sub_tab: CalSubTab::AutoCal,
            edit: EditBuffers::default(),
            auto_cal: AutoCalState::default(),
            table_cal: TableCalState::default(),
            view_mode: ViewModeState::default(),
            mobile_export: MobileExportState::default(),
            pending_reads: HashSet::new(),
            pending_writes: HashSet::new(),
            pending_adv_reads: HashSet::new(),
            pending_adv_writes: HashSet::new(),
        }
    }
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum Tab {
    #[default]
    Connect,
    Configuration,
    Calibration,
    Log,
    Live,
    MobileExport,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum CalSubTab {
    #[default]
    AutoCal,
    TableCal,
    Advanced,
}

/// Edit buffers for text input fields
#[derive(Default)]
pub struct EditBuffers {
    // Configuration
    pub config_pin: String,
    pub data_rate: String,
    pub resolution: String,
    pub battery_threshold: String,
    pub view_pin: String,
    pub data_tag: String,
    pub system_zero: String,
    // Calibration
    pub sensitivity_range: String,
    pub lin_index: String,
    pub lin_repeat: String,
    pub lin_points: String,
    pub data_gain: String,
    pub data_offset: String,
    pub cal_pin: String,
    pub cal_units: String,
    pub cal_units_display: String,
    // Advanced
    pub adv_index: String,
    pub adv_data: String,
}

// ── Auto Calibration ──────────────────────────────────────────────

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum CaptureStatus {
    #[default]
    NotCaptured,
    Capturing,
    Captured,
}

pub struct AutoCalPoint {
    pub target_value: String,
    pub base_value: Option<f32>,
    pub capture_status: CaptureStatus,
}

impl Default for AutoCalPoint {
    fn default() -> Self {
        Self {
            target_value: String::new(),
            base_value: None,
            capture_status: CaptureStatus::NotCaptured,
        }
    }
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum AutoCalPhase {
    #[default]
    Editing,
    Applied,
}

pub struct AutoCalState {
    pub phase: AutoCalPhase,
    pub points: Vec<AutoCalPoint>,
    pub capturing_index: Option<usize>,
    pub error: Option<String>,
    pub has_been_applied: bool,
}

impl Default for AutoCalState {
    fn default() -> Self {
        Self {
            phase: AutoCalPhase::Editing,
            points: vec![
                AutoCalPoint {
                    target_value: "0".to_string(),
                    base_value: None,
                    capture_status: CaptureStatus::NotCaptured,
                },
                AutoCalPoint::default(),
            ],
            capturing_index: None,
            error: None,
            has_been_applied: false,
        }
    }
}

// ── Table Calibration ──────────────────────────────────────────────

pub struct TableCalRow {
    pub mv_per_v: String,   // raw mV/V input
    pub eng_value: String,  // engineering value input
}

impl Default for TableCalRow {
    fn default() -> Self {
        Self {
            mv_per_v: String::new(),
            eng_value: String::new(),
        }
    }
}

pub struct TableCalState {
    pub rows: Vec<TableCalRow>,
    pub applied: bool,
    pub error: Option<String>,
}

impl Default for TableCalState {
    fn default() -> Self {
        Self {
            rows: vec![TableCalRow::default(), TableCalRow::default()],
            applied: false,
            error: None,
        }
    }
}

// ── View Mode ─────────────────────────────────────────────────────

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum ViewSource {
    #[default]
    Advertising,  // From scan advertising packets (no connection needed)
    Connected,    // From BLE notifications (live data)
}

pub struct ViewModeState {
    pub active: bool,
    pub source: ViewSource,
    pub peripheral_id: Option<String>,  // device being viewed (advertising mode)
    pub device_name: String,            // cached device name for display
    pub view_pin: String,               // entered View PIN
    pub show_pin_dialog: bool,          // show PIN entry dialog
    pub current_value: Option<f32>,
    pub current_units: Option<u8>,
    pub current_status: Option<StatusByte>,
    pub data_tag: Option<u16>,
    pub history: VecDeque<(f64, f32)>,
    pub start_time: Option<std::time::Instant>,
    pub last_update: Option<std::time::Instant>,
}

impl Default for ViewModeState {
    fn default() -> Self {
        Self {
            active: false,
            source: ViewSource::Advertising,
            peripheral_id: None,
            device_name: String::new(),
            view_pin: String::new(),
            show_pin_dialog: false,
            current_value: None,
            current_units: None,
            current_status: None,
            data_tag: None,
            history: VecDeque::new(),
            start_time: None,
            last_update: None,
        }
    }
}

// ── Mobile App Export ─────────────────────────────────────────────

pub struct MobileExportRow {
    pub data_tag: String,
    pub description: String,
    pub unit_byte: Option<u8>,
}

impl Default for MobileExportRow {
    fn default() -> Self {
        Self {
            data_tag: String::new(),
            description: String::new(),
            unit_byte: None,
        }
    }
}

pub struct MobileExportState {
    pub project_name: String,
    pub view_pin: String,
    pub timeout_index: usize,
    pub transmitters: Vec<MobileExportRow>,
    pub decimal_places: usize,
    pub last_json: String,
    pub populated_from_device: bool,
}

impl Default for MobileExportState {
    fn default() -> Self {
        Self {
            project_name: String::new(),
            view_pin: "0000".to_string(),
            timeout_index: 2, // 12 seconds default
            transmitters: vec![MobileExportRow::default()],
            decimal_places: 2,
            last_json: String::new(),
            populated_from_device: false,
        }
    }
}
