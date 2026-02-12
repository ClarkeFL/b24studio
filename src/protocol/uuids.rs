use uuid::Uuid;

/// Construct a B24 UUID from the short identifier.
/// Base: XXXXXXXX-a0e8-11e6-bdf4-0800200c9a66
fn b24_uuid(short: u32) -> Uuid {
    Uuid::from_fields(
        short,
        0xa0e8,
        0x11e6,
        &[0xbd, 0xf4, 0x08, 0x00, 0x20, 0x0c, 0x9a, 0x66],
    )
}

// ── Standard BLE GAP ──────────────────────────────────────────────

/// Standard BLE GAP Device Name characteristic (0x2A00)
pub fn char_gap_device_name() -> Uuid {
    Uuid::from_fields(0x00002a00, 0x0000, 0x1000, &[0x80, 0x00, 0x00, 0x80, 0x5f, 0x9b, 0x34, 0xfb])
}

// ── Configuration Service ──────────────────────────────────────────

pub fn svc_config() -> Uuid { b24_uuid(0xa970fd30) }
pub fn char_data_rate() -> Uuid { b24_uuid(0xa970fd31) }
pub fn char_resolution() -> Uuid { b24_uuid(0xa970fd32) }
pub fn char_battery_thresh() -> Uuid { b24_uuid(0xa970fd33) }
pub fn char_view_pin() -> Uuid { b24_uuid(0xa970fd34) }
pub fn char_serial_number() -> Uuid { b24_uuid(0xa970fd35) }
pub fn char_data_tag() -> Uuid { b24_uuid(0xa970fd36) }
pub fn char_battery_value() -> Uuid { b24_uuid(0xa970fd37) }
pub fn char_system_zero() -> Uuid { b24_uuid(0xa970fd38) }
pub fn char_config_pin() -> Uuid { b24_uuid(0xa970fd39) }
pub fn char_model_name() -> Uuid { b24_uuid(0xa970fd3a) }
pub fn char_firmware_ver() -> Uuid { b24_uuid(0xa970fd3b) }

// ── Data Service ───────────────────────────────────────────────────

pub fn svc_data() -> Uuid { b24_uuid(0xa9712440) }
pub fn char_status() -> Uuid { b24_uuid(0xa9712441) }
pub fn char_data_value() -> Uuid { b24_uuid(0xa9712442) }
pub fn char_data_units() -> Uuid { b24_uuid(0xa9712443) }

// ── Calibration Characteristics ────────────────────────────────────

pub fn char_sens_range() -> Uuid { b24_uuid(0xa9717261) }
pub fn char_coeff_at_idx() -> Uuid { b24_uuid(0xa9717262) }
pub fn char_lin_index() -> Uuid { b24_uuid(0xa9717263) }
pub fn char_lin_repeat() -> Uuid { b24_uuid(0xa9717264) }
pub fn char_lin_points() -> Uuid { b24_uuid(0xa9717265) }
pub fn char_base_value() -> Uuid { b24_uuid(0xa9717266) }
pub fn char_base_units() -> Uuid { b24_uuid(0xa9717267) }
pub fn char_data_gain() -> Uuid { b24_uuid(0xa9717268) }
pub fn char_data_offset() -> Uuid { b24_uuid(0xa9717269) }
pub fn char_cal_pin() -> Uuid { b24_uuid(0xa971726a) }
pub fn char_cal_units() -> Uuid { b24_uuid(0xa971726b) }
pub fn char_adv_index() -> Uuid { b24_uuid(0xa971726c) }
pub fn char_adv_data() -> Uuid { b24_uuid(0xa971726d) }

/// All configuration characteristic UUIDs for "Read All Registers"
pub fn all_config_uuids() -> Vec<Uuid> {
    vec![
        char_config_pin(),
        char_data_rate(),
        char_resolution(),
        char_battery_thresh(),
        char_view_pin(),
        char_serial_number(),
        char_data_tag(),
        char_battery_value(),
        char_system_zero(),
        char_model_name(),
        char_firmware_ver(),
    ]
}

/// All calibration characteristic UUIDs for "Read All Registers"
pub fn all_calibration_uuids() -> Vec<Uuid> {
    vec![
        char_sens_range(),
        char_coeff_at_idx(),
        char_lin_index(),
        char_lin_repeat(),
        char_lin_points(),
        char_base_value(),
        char_base_units(),
        char_data_gain(),
        char_data_offset(),
        char_cal_pin(),
        char_cal_units(),
    ]
}
