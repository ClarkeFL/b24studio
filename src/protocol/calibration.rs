use super::types::LinearisationEntry;

/// Compute gain and offset for a two-point calibration.
///
/// - `low_base` / `high_base`: raw base values (mV/V) read from the device
/// - `low_target` / `high_target`: desired engineering values entered by the user
///
/// Returns (gain, offset) or None if the base values are equal.
pub fn two_point_calibration(
    low_base: f32,
    high_base: f32,
    low_target: f32,
    high_target: f32,
) -> Option<(f32, f32)> {
    let base_diff = high_base - low_base;
    if base_diff.abs() < 1e-12 {
        return None;
    }
    let gain = (high_target - low_target) / base_diff;
    let offset = low_target - gain * low_base;
    Some((gain, offset))
}

/// Build a single-segment linearisation table entry from a two-point calibration
pub fn build_linearisation_entry(
    index: u8,
    valid_from: f32,
    valid_to: f32,
    gain: f32,
    offset: f32,
) -> LinearisationEntry {
    LinearisationEntry {
        index,
        valid_from,
        gain,
        offset,
        valid_to,
    }
}

/// The linearisation table is stored as a flat array of f32 coefficients.
/// Each entry has 3 coefficients: [valid_from, gain, offset]
/// The final entry's valid_to is the next entry's valid_from (or the full-scale value).
///
/// To read the full table:
/// 1. Read CHAR_LIN_POINTS to get point count
/// 2. For each point index 0..points:
///    a. Write index * 3 to CHAR_LIN_INDEX
///    b. Read CHAR_COEFF_AT_IDX for valid_from
///    c. Write index * 3 + 1 to CHAR_LIN_INDEX
///    d. Read CHAR_COEFF_AT_IDX for gain
///    e. Write index * 3 + 2 to CHAR_LIN_INDEX
///    f. Read CHAR_COEFF_AT_IDX for offset
/// 3. Valid_to for entry N = valid_from of entry N+1 (or full-scale for last)
///
/// Returns the sequence of (lin_index_to_write, is_read) operations.
pub fn linearisation_read_sequence(num_points: u8) -> Vec<(u8, &'static str)> {
    let mut ops = Vec::new();
    for i in 0..num_points {
        let base = i * 3;
        ops.push((base, "valid_from"));
        ops.push((base + 1, "gain"));
        ops.push((base + 2, "offset"));
    }
    // Read the final valid_to (which is at index num_points * 3)
    ops.push((num_points * 3, "final_valid_to"));
    ops
}
