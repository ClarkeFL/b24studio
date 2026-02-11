use std::collections::{VecDeque, HashMap};
use chrono::{DateTime, Local};
use uuid::Uuid;

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

// ── Connection ─────────────────────────────────────────────────────

#[derive(Default)]
pub struct ConnectionState {
    pub phase: ConnectionPhase,
    pub scanned_devices: Vec<ScannedDevice>,
    pub config_pin: String,
    pub selected_device_index: Option<usize>,
    pub error_message: Option<String>,
    pub status_text: String,
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
}

// ── Configuration Registers ────────────────────────────────────────

#[derive(Default)]
pub struct ConfigRegisters {
    pub config_pin: Option<u32>,
    pub data_rate: Option<u32>,
    pub resolution: Option<u8>,
    pub battery_threshold: Option<f32>,
    pub view_pin: Option<String>,
    pub serial_number: Option<String>,
    pub data_tag: Option<String>,
    pub battery_value: Option<f32>,
    pub system_zero: Option<f32>,
    pub model_name: Option<String>,
    pub firmware_version: Option<String>,
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
    pub cal_wizard: CalWizardState,
    pub pending_reads: Vec<Uuid>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active_tab: Tab::Connect,
            cal_sub_tab: CalSubTab::AutoCal,
            edit: EditBuffers::default(),
            cal_wizard: CalWizardState::default(),
            pending_reads: Vec::new(),
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
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum CalSubTab {
    #[default]
    AutoCal,
    Multipoint,
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

#[derive(Default)]
pub struct CalWizardState {
    pub low_value: String,
    pub high_value: String,
    pub low_acquired: Option<f32>,
    pub high_acquired: Option<f32>,
    pub current_point: u32,
}
