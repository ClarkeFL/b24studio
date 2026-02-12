use eframe::egui;
use uuid::Uuid;
use crate::state::{AppState, CalSubTab, CaptureStatus, AutoCalPhase, AutoCalPoint, TableCalRow};
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::{uuids, codec};
use crate::protocol::types::{DeviceAction, AdvancedParam, DataUnits, ParamType};
use crate::protocol::calibration;
use crate::ui::widgets;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    ui.add_space(8.0);

    // Header with sub-tabs + Export/Import (matching Configuration tab style)
    ui.horizontal(|ui| {
        widgets::page_header(ui, "Calibration");
        ui.add_space(16.0);
        ui.selectable_value(&mut state.ui.cal_sub_tab, CalSubTab::AutoCal, "Auto Calibration");
        ui.selectable_value(&mut state.ui.cal_sub_tab, CalSubTab::TableCal, "Table Calibration");
        ui.selectable_value(&mut state.ui.cal_sub_tab, CalSubTab::Advanced, "Advanced / Registers");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_sized([100.0, 32.0], egui::Button::new(
                egui::RichText::new("Import").size(15.0)
            )).clicked() {
                import_calibration(state, ble);
            }
            ui.add_space(4.0);
            if ui.add_sized([100.0, 32.0], egui::Button::new(
                egui::RichText::new("Export").size(15.0)
            )).clicked() {
                export_calibration(state);
            }
        });
    });
    ui.separator();

    match state.ui.cal_sub_tab {
        CalSubTab::AutoCal => show_auto_cal(ui, state, ble),
        CalSubTab::TableCal => show_table_cal(ui, state, ble),
        CalSubTab::Advanced => show_advanced(ui, state, ble),
    }
}

// -- Helpers ----------------------------------------------------------------

fn send_tracked(state: &mut AppState, ble: &BleHandle, cmd: BleCommand) {
    state.track_send(&cmd);
    ble.send(cmd);
}

fn value_or_spinner(ui: &mut egui::Ui, text: &str, uuid: Uuid, state: &AppState) {
    if state.is_pending(&uuid) {
        ui.add(egui::Spinner::new().size(widgets::SPINNER_SIZE));
    } else {
        ui.monospace(text);
    }
}

fn adv_value_or_spinner(ui: &mut egui::Ui, text: &str, index: u8, state: &AppState) {
    if state.is_adv_pending(index) {
        ui.add(egui::Spinner::new().size(widgets::SPINNER_SIZE));
    } else {
        ui.monospace(text);
    }
}

// -- Fetch Existing Calibration ---------------------------------------------

/// Show a bar with "Fetch Existing Calibration" button + Refresh,
/// styled to match the live value banner.
/// `is_table_cal` determines which form to populate.
fn show_fetch_calibration_bar(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle, is_table_cal: bool) {
    let dark = ui.visuals().dark_mode;
    egui::Frame::none()
        .fill(widgets::subtle_bg(dark))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let has_table = !state.calibration.linearisation_table.is_empty();
                ui.add_enabled_ui(has_table, |ui| {
                    if ui.add_sized([200.0, 28.0], egui::Button::new(
                        egui::RichText::new("Fetch Existing Calibration").size(14.0)
                    )).clicked() {
                        if is_table_cal {
                            populate_table_cal_from_device(state);
                        } else {
                            populate_auto_cal_from_device(state);
                        }
                    }
                });
                if !has_table {
                    ui.label(
                        egui::RichText::new("No linearisation data on device yet")
                            .size(13.0).color(widgets::muted_text(dark))
                    );
                }
                // Refresh button to re-read linearisation table from device
                if ui.add_sized([80.0, 28.0], egui::Button::new(
                    egui::RichText::new("↻ Refresh").size(13.0)
                )).clicked() {
                    send_tracked(state, ble, BleCommand::ReadAll(uuids::all_calibration_uuids()));
                    send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_lin_points()));
                }

                if has_table {
                    ui.separator();
                    let n = state.calibration.linearisation_table.len();
                    ui.label(
                        egui::RichText::new(format!("{n} segment{} on device", if n == 1 { "" } else { "s" }))
                            .size(13.0).color(widgets::muted_text(dark))
                    );
                }
            });
        });
}

/// Reverse-engineer calibration points from the device's linearisation table
/// and populate the Auto Cal form.
///
/// Each linearisation segment stores (valid_from, gain, offset, valid_to).
/// The B24 formula is: eng_value = gain * raw_mv - offset
/// So at each mV/V breakpoint we can compute the engineering value.
fn populate_auto_cal_from_device(state: &mut AppState) {
    let table = &state.calibration.linearisation_table;
    if table.is_empty() { return; }

    // Build calibration points: each segment boundary gives us (mV/V, eng_value)
    let mut points: Vec<(f32, f32)> = Vec::new();

    for (i, entry) in table.iter().enumerate() {
        // First point of this segment
        let eng_at_from = entry.gain * entry.valid_from - entry.offset;
        if i == 0 || (points.last().map(|p| (p.0 - entry.valid_from).abs() > 1e-10).unwrap_or(true)) {
            points.push((entry.valid_from, eng_at_from));
        }

        // Last segment also contributes its valid_to endpoint
        if i == table.len() - 1 {
            let eng_at_to = entry.gain * entry.valid_to - entry.offset;
            points.push((entry.valid_to, eng_at_to));
        }
    }

    // Populate auto cal points
    state.ui.auto_cal.points = points.iter().map(|(mv, eng)| {
        AutoCalPoint {
            target_value: format!("{eng:.6}"),
            base_value: Some(*mv),
            capture_status: CaptureStatus::Captured,
        }
    }).collect();
    state.ui.auto_cal.phase = AutoCalPhase::Editing;
    state.ui.auto_cal.has_been_applied = false;
    state.ui.auto_cal.error = None;
}

/// Reverse-engineer calibration points from the device's linearisation table
/// and populate the Table Cal form.
fn populate_table_cal_from_device(state: &mut AppState) {
    let table = &state.calibration.linearisation_table;
    if table.is_empty() { return; }

    // Build calibration points: each segment boundary gives us (mV/V, eng_value)
    let mut points: Vec<(f32, f32)> = Vec::new();

    for (i, entry) in table.iter().enumerate() {
        let eng_at_from = entry.gain * entry.valid_from - entry.offset;
        if i == 0 || (points.last().map(|p| (p.0 - entry.valid_from).abs() > 1e-10).unwrap_or(true)) {
            points.push((entry.valid_from, eng_at_from));
        }

        if i == table.len() - 1 {
            let eng_at_to = entry.gain * entry.valid_to - entry.offset;
            points.push((entry.valid_to, eng_at_to));
        }
    }

    // Populate table cal rows
    state.ui.table_cal.rows = points.iter().map(|(mv, eng)| {
        TableCalRow {
            mv_per_v: format!("{mv:.6}"),
            eng_value: format!("{eng:.6}"),
        }
    }).collect();
    state.ui.table_cal.applied = false;
    state.ui.table_cal.error = None;
}

// -- Auto Calibration -------------------------------------------------------

fn show_auto_cal(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);

        // Live value banner (full width)
        show_live_value_banner(ui, state);

        ui.add_space(4.0);

        // Fetch Existing Calibration button
        show_fetch_calibration_bar(ui, state, ble, false);

        ui.add_space(8.0);

        // Two-column layout: left = points + actions, right = preview
        ui.columns(2, |cols| {
            // LEFT: Calibration Points
            widgets::section_header(&mut cols[0], "Calibration Points");

            cols[0].label(
                egui::RichText::new(
                    "Enter target values, then capture raw mV/V at each load.\n\
                     Re-capture any point by clicking Capture again."
                ).size(14.0)
            );
            cols[0].add_space(8.0);

            // Points table
            show_points_table(&mut cols[0], state, ble);

            // Action buttons
            show_action_buttons(&mut cols[0], state, ble);

            // Status messages
            if state.ui.auto_cal.phase == AutoCalPhase::Applied {
                cols[0].add_space(8.0);
                cols[0].label(
                    egui::RichText::new("Calibration written to device and Calculate Coefficients triggered.")
                        .size(15.0)
                        .color(widgets::COLOR_SUCCESS)
                );
            }

            if let Some(err) = &state.ui.auto_cal.error {
                cols[0].add_space(8.0);
                cols[0].label(
                    egui::RichText::new(err)
                        .color(widgets::COLOR_ERROR)
                );
            }

            // RIGHT: Preview segments
            show_preview(&mut cols[1], state);
        });
    });
}

fn show_live_value_banner(ui: &mut egui::Ui, state: &AppState) {
    let dark = ui.visuals().dark_mode;
    egui::Frame::none()
        .fill(widgets::subtle_bg(dark))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Calibrated live value
                let cal_value = state.live_data.current_value
                    .map(|v| format!("{v:.4}"))
                    .unwrap_or_else(|| "--".to_string());
                let units = state.calibration.cal_units
                    .or(state.live_data.current_units)
                    .map(|u| DataUnits::from_byte(u).label().to_string())
                    .unwrap_or_default();

                ui.label(egui::RichText::new("Live Value:").size(16.0).strong());
                ui.label(egui::RichText::new(format!("{cal_value}  {units}"))
                    .size(20.0).monospace()
                    .color(widgets::COLOR_SUCCESS));

                ui.add_space(32.0);

                // Raw mV/V
                let raw = state.calibration.base_value
                    .map(|v| format!("{v:.6}"))
                    .unwrap_or_else(|| "--".to_string());
                ui.label(egui::RichText::new("Raw mV/V:").size(14.0)
                    .color(widgets::muted_text(dark)));
                ui.label(egui::RichText::new(raw).size(14.0).monospace()
                    .color(widgets::muted_text(dark)));
            });
        });
}

fn show_points_table(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    let any_capturing = state.ui.auto_cal.capturing_index.is_some();
    let point_count = state.ui.auto_cal.points.len();

    // Collect capture and clear actions to apply after the grid
    let mut capture_idx: Option<usize> = None;
    let mut clear_idx: Option<usize> = None;

    egui::Grid::new("auto_cal_points")
        .num_columns(6)
        .spacing([12.0, 6.0])
        .striped(true)
        .min_col_width(40.0)
        .show(ui, |ui| {
            ui.strong("#");
            ui.horizontal(|ui| {
                ui.strong("Target Value");
                widgets::help_icon(ui, "The engineering value (in calibration units) you want\nthe transmitter to output at this load point.\nE.g. 0 for zero load, 100 for full load.");
            });
            ui.horizontal(|ui| {
                ui.strong("Raw mV/V");
                widgets::help_icon(ui, "The raw sensor signal captured from the strain gauge bridge.\nApply the known load, then click Capture to read this value.");
            });
            ui.strong("Status");
            ui.strong(""); // Capture
            ui.strong(""); // Clear
            ui.end_row();

            for (i, point) in state.ui.auto_cal.points.iter_mut().enumerate() {
                ui.label(egui::RichText::new(format!("{}", i + 1)).size(15.0));

                // Target value input
                let changed = ui.add_sized([120.0, 28.0],
                    egui::TextEdit::singleline(&mut point.target_value)
                        .hint_text(if i == 0 { "0.0" } else { "e.g. 100.0" })
                        .vertical_align(egui::Align::Center)
                ).changed();
                if changed {
                    // Reset applied state when values change
                    state.ui.auto_cal.phase = AutoCalPhase::Editing;
                    state.ui.auto_cal.error = None;
                }

                // Raw mV/V display
                match point.capture_status {
                    CaptureStatus::Capturing => {
                        ui.add(egui::Spinner::new().size(widgets::SPINNER_SIZE));
                    }
                    CaptureStatus::Captured => {
                        let val = point.base_value.unwrap_or(0.0);
                        ui.label(egui::RichText::new(format!("{val:.6}")).size(14.0).monospace()
                            .color(widgets::COLOR_SUCCESS));
                    }
                    CaptureStatus::NotCaptured => {
                        ui.monospace("--");
                    }
                }

                // Status
                let dark = ui.visuals().dark_mode;
                let (status_text, status_color) = match point.capture_status {
                    CaptureStatus::NotCaptured => ("Not captured", widgets::muted_text(dark)),
                    CaptureStatus::Capturing => ("Capturing...", widgets::COLOR_WARNING),
                    CaptureStatus::Captured => ("Captured", widgets::COLOR_SUCCESS),
                };
                ui.label(egui::RichText::new(status_text).size(13.0).color(status_color));

                // Capture button
                ui.add_enabled_ui(!any_capturing, |ui| {
                    if ui.add_sized([80.0, 28.0], egui::Button::new(
                        egui::RichText::new("Capture").size(14.0).color(egui::Color32::WHITE)
                    ).fill(egui::Color32::from_rgb(50, 100, 180))).clicked() {
                        capture_idx = Some(i);
                    }
                });

                // Clear button
                if ui.small_button("Clear").clicked() {
                    clear_idx = Some(i);
                }

                ui.end_row();
            }
        });

    // Apply capture action
    if let Some(i) = capture_idx {
        state.ui.auto_cal.points[i].capture_status = CaptureStatus::Capturing;
        state.ui.auto_cal.capturing_index = Some(i);
        state.ui.auto_cal.phase = AutoCalPhase::Editing;
        state.ui.auto_cal.error = None;
        send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_base_value()));
    }

    // Apply clear action
    if let Some(i) = clear_idx {
        state.ui.auto_cal.points[i].base_value = None;
        state.ui.auto_cal.points[i].capture_status = CaptureStatus::NotCaptured;
        state.ui.auto_cal.phase = AutoCalPhase::Editing;
        state.ui.auto_cal.error = None;
    }

    ui.add_space(8.0);

    // Add/Remove row controls
    let mut remove_last = false;
    ui.horizontal(|ui| {
        if ui.button("+ Add Point").clicked() && point_count < 16 {
            state.ui.auto_cal.points.push(AutoCalPoint::default());
            state.ui.auto_cal.phase = AutoCalPhase::Editing;
        }
        if point_count > 2 {
            if ui.button("- Remove Last").clicked() {
                remove_last = true;
            }
        }
        ui.add_space(8.0);
        let dark = ui.visuals().dark_mode;
        ui.label(
            egui::RichText::new(format!("{} / 16 points", point_count))
                .size(13.0).color(widgets::muted_text(dark))
        );
    });
    if remove_last {
        state.ui.auto_cal.points.pop();
        state.ui.auto_cal.phase = AutoCalPhase::Editing;
    }
}

fn show_preview(ui: &mut egui::Ui, state: &AppState) {
    // Only show preview if all points are captured and valid
    let segments = compute_preview_segments(&state.ui.auto_cal);
    let segments = match segments {
        Some(s) if !s.is_empty() => s,
        _ => return,
    };

    ui.add_space(12.0);
    widgets::section_header(ui, "Preview");

    egui::Grid::new("auto_cal_preview")
        .num_columns(5)
        .spacing([16.0, 4.0])
        .striped(true)
        .min_col_width(80.0)
        .show(ui, |ui| {
            ui.strong("Segment");
            ui.strong("Valid From");
            ui.strong("Gain");
            ui.strong("Offset");
            ui.strong("Valid To");
            ui.end_row();

            for (i, seg) in segments.iter().enumerate() {
                ui.label(format!("{}", i + 1));
                ui.monospace(format!("{:.6}", seg.0));
                ui.monospace(format!("{:.6}", seg.1));
                ui.monospace(format!("{:.6}", seg.2));
                ui.monospace(format!("{:.6}", seg.3));
                ui.end_row();
            }
        });
}

/// Returns segments as (valid_from, gain, offset, valid_to)
fn compute_preview_segments(auto_cal: &crate::state::AutoCalState) -> Option<Vec<(f32, f32, f32, f32)>> {
    let mut pairs: Vec<(f32, f32)> = Vec::new();
    for point in &auto_cal.points {
        let target: f32 = point.target_value.parse().ok()?;
        let base = point.base_value?;
        pairs.push((base, target));
    }

    if pairs.len() < 2 {
        return None;
    }

    // Sort by raw mV/V ascending
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut segments = Vec::new();
    for window in pairs.windows(2) {
        let (low_base, low_target) = window[0];
        let (high_base, high_target) = window[1];
        if let Some((gain, offset)) = calibration::two_point_calibration(
            low_base, high_base, low_target, high_target
        ) {
            segments.push((low_base, gain, offset, high_base));
        }
    }

    Some(segments)
}

fn show_action_buttons(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    ui.add_space(16.0);

    // Check if apply is possible
    let all_captured = state.ui.auto_cal.points.iter()
        .all(|p| p.capture_status == CaptureStatus::Captured);
    let all_valid_targets = state.ui.auto_cal.points.iter()
        .all(|p| p.target_value.parse::<f32>().is_ok());
    let enough_points = state.ui.auto_cal.points.len() >= 2;
    let can_apply = all_captured && all_valid_targets && enough_points
        && state.ui.auto_cal.capturing_index.is_none();

    ui.horizontal(|ui| {
        let btn_text = if state.ui.auto_cal.has_been_applied {
            "Re-apply to Device"
        } else {
            "Apply to Device"
        };

        ui.add_enabled_ui(can_apply, |ui| {
            if ui.add_sized([220.0, 36.0], egui::Button::new(
                egui::RichText::new(btn_text).size(16.0).color(egui::Color32::WHITE)
            ).fill(widgets::COLOR_BTN_GREEN)).clicked() {
                apply_auto_calibration(state, ble);
            }
        });

        ui.add_space(16.0);

        if ui.add_sized([100.0, 36.0], egui::Button::new("Reset All")).clicked() {
            state.ui.auto_cal = Default::default();
        }

        // Hint text if not all ready
        if !can_apply && enough_points {
            ui.add_space(8.0);
            if !all_captured {
                let dark = ui.visuals().dark_mode;
                ui.label(egui::RichText::new("Capture all points first")
                    .size(13.0).color(widgets::muted_text(dark)));
            } else if !all_valid_targets {
                ui.label(egui::RichText::new("All target values must be valid numbers")
                    .size(13.0).color(widgets::COLOR_WARNING));
            }
        }
    });
}

fn apply_auto_calibration(state: &mut AppState, ble: &BleHandle) {
    // 1. Collect and validate all points
    let mut pairs: Vec<(f32, f32)> = Vec::new();
    for point in &state.ui.auto_cal.points {
        let target: f32 = match point.target_value.parse() {
            Ok(v) => v,
            Err(_) => {
                state.ui.auto_cal.error = Some("All target values must be valid numbers.".to_string());
                return;
            }
        };
        let base = match point.base_value {
            Some(v) => v,
            None => {
                state.ui.auto_cal.error = Some("All points must be captured before applying.".to_string());
                return;
            }
        };
        pairs.push((base, target));
    }

    if pairs.len() < 2 {
        state.ui.auto_cal.error = Some("At least 2 points are required.".to_string());
        return;
    }

    // 2. Sort by raw mV/V ascending
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // 3. Check for duplicate base values
    for window in pairs.windows(2) {
        if (window[1].0 - window[0].0).abs() < 1e-12 {
            state.ui.auto_cal.error = Some("Two points have identical raw mV/V values. Re-capture one of them.".to_string());
            return;
        }
    }

    let num_segments = pairs.len() - 1;

    // 4. Write to device
    // Set lin_repeat = 3 (valid_from, gain, offset per segment)
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_lin_repeat(),
        data: codec::encode_u8(3),
    });

    // Write each segment
    for (seg_idx, window) in pairs.windows(2).enumerate() {
        let (low_mv, low_eng) = window[0];
        let (high_mv, high_eng) = window[1];

        let (gain, offset) = calibration::two_point_calibration(
            low_mv, high_mv, low_eng, high_eng
        ).unwrap_or((1.0, 0.0));

        let base_idx = (seg_idx as u8) * 3;

        // valid_from
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_lin_index(),
            data: codec::encode_u8(base_idx),
        });
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_coeff_at_idx(),
            data: codec::encode_f32_be(low_mv),
        });

        // gain
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_lin_index(),
            data: codec::encode_u8(base_idx + 1),
        });
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_coeff_at_idx(),
            data: codec::encode_f32_be(gain),
        });

        // offset
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_lin_index(),
            data: codec::encode_u8(base_idx + 2),
        });
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_coeff_at_idx(),
            data: codec::encode_f32_be(offset),
        });
    }

    // Write final valid_to
    let last_mv = pairs.last().unwrap().0;
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_lin_index(),
        data: codec::encode_u8((num_segments as u8) * 3),
    });
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_coeff_at_idx(),
        data: codec::encode_f32_be(last_mv),
    });

    // Set lin_points
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_lin_points(),
        data: codec::encode_u8(num_segments as u8),
    });

    // Set data_gain = 1.0, data_offset = 0.0
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_data_gain(),
        data: codec::encode_f32_be(1.0),
    });
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_data_offset(),
        data: codec::encode_f32_be(0.0),
    });

    // Trigger Calculate Coefficients
    ble.send(BleCommand::ExecuteAction(DeviceAction::CalculateCoefficients));

    state.ui.auto_cal.phase = AutoCalPhase::Applied;
    state.ui.auto_cal.has_been_applied = true;
    state.ui.auto_cal.error = None;
}

// -- Calibration Export / Import ---------------------------------------------

fn export_calibration(state: &AppState) {
    let mut lines: Vec<String> = Vec::new();

    lines.push("# B24 Calibration Export".to_string());
    lines.push(format!("# Exported: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    if let Some(ref model) = state.config.model_name {
        lines.push(format!("# Model: {}", model));
    }
    if let Some(sn) = state.config.serial_number {
        lines.push(format!("# Serial: {}", sn));
    }
    lines.push(String::new());

    // Calibration settings
    lines.push("[Settings]".to_string());
    if let Some(v) = state.calibration.sensitivity_range {
        lines.push(format!("sensitivity_range = {}", v));
    }
    if let Some(v) = state.calibration.data_gain {
        lines.push(format!("data_gain = {}", v));
    }
    if let Some(v) = state.calibration.data_offset {
        lines.push(format!("data_offset = {}", v));
    }
    if let Some(v) = state.calibration.cal_units {
        let du = DataUnits::from_byte(v);
        lines.push(format!("cal_units = {} # {}", v, du.label()));
    }
    if let Some(v) = state.calibration.lin_points {
        lines.push(format!("lin_points = {}", v));
    }
    if let Some(v) = state.calibration.lin_repeat {
        lines.push(format!("lin_repeat = {}", v));
    }
    lines.push(String::new());

    // Auto Cal points (if any were captured)
    let has_auto_points = state.ui.auto_cal.points.iter()
        .any(|p| p.capture_status == CaptureStatus::Captured);
    if has_auto_points {
        lines.push("[Calibration Points]".to_string());
        lines.push("# mV/V, Engineering Value".to_string());
        for point in &state.ui.auto_cal.points {
            if let (Some(base), Ok(target)) = (point.base_value, point.target_value.parse::<f32>()) {
                lines.push(format!("{}, {}", base, target));
            }
        }
        lines.push(String::new());
    }

    // Table Cal points (if any were entered)
    let has_table_points = state.ui.table_cal.rows.iter()
        .any(|r| !r.mv_per_v.is_empty() && !r.eng_value.is_empty());
    if has_table_points {
        lines.push("[Table Points]".to_string());
        lines.push("# mV/V, Engineering Value".to_string());
        for row in &state.ui.table_cal.rows {
            if let (Ok(mv), Ok(eng)) = (row.mv_per_v.parse::<f32>(), row.eng_value.parse::<f32>()) {
                lines.push(format!("{}, {}", mv, eng));
            }
        }
        lines.push(String::new());
    }

    // Linearisation table (from device)
    if !state.calibration.linearisation_table.is_empty() {
        lines.push("[Linearisation Table]".to_string());
        lines.push("# Index, Valid From, Gain, Offset, Valid To".to_string());
        for entry in &state.calibration.linearisation_table {
            lines.push(format!("{}, {}, {}, {}, {}", entry.index, entry.valid_from, entry.gain, entry.offset, entry.valid_to));
        }
        lines.push(String::new());
    }

    let content = lines.join("\n");
    let tag = state.config.data_tag.as_deref().unwrap_or("0000");
    let filename = format!("{}_calibration_export.txt", tag);

    let task = rfd::FileDialog::new()
        .set_title("Export B24 Calibration")
        .add_filter("Text files", &["txt"])
        .set_file_name(&filename)
        .save_file();

    if let Some(path) = task {
        let _ = std::fs::write(path, content);
    }
}

fn import_calibration(state: &mut AppState, ble: &BleHandle) {
    let task = rfd::FileDialog::new()
        .set_title("Import B24 Calibration")
        .add_filter("Text files", &["txt"])
        .pick_file();

    let Some(path) = task else { return; };
    let Ok(data) = std::fs::read_to_string(&path) else { return; };

    // Parse the file looking for calibration points or table points
    let mut current_section = String::new();
    let mut cal_points: Vec<(f32, f32)> = Vec::new();
    let mut settings: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len()-1].to_lowercase();
            continue;
        }

        match current_section.as_str() {
            "settings" => {
                if let Some((key, val)) = line.split_once('=') {
                    let val = val.split('#').next().unwrap_or(val); // strip inline comments
                    settings.insert(key.trim().to_string(), val.trim().to_string());
                }
            }
            "calibration points" | "table points" => {
                // Parse "mV/V, engineering value" lines
                if let Some((mv_str, eng_str)) = line.split_once(',') {
                    if let (Ok(mv), Ok(eng)) = (mv_str.trim().parse::<f32>(), eng_str.trim().parse::<f32>()) {
                        cal_points.push((mv, eng));
                    }
                }
            }
            _ => {}
        }
    }

    // Write settings
    if let Some(v) = settings.get("sensitivity_range").and_then(|s| s.parse::<u8>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_sens_range(), data: codec::encode_u8(v),
        });
    }
    if let Some(v) = settings.get("cal_units").and_then(|s| s.parse::<u8>().ok()) {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_cal_units(), data: codec::encode_u8(v),
        });
    }

    // Write calibration points as linearisation table
    if cal_points.len() >= 2 {
        // Sort by mV/V ascending
        cal_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let num_segments = cal_points.len() - 1;

        // Set lin_repeat = 3
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_lin_repeat(), data: codec::encode_u8(3),
        });

        // Write each segment
        for (seg_idx, window) in cal_points.windows(2).enumerate() {
            let (low_mv, low_eng) = window[0];
            let (high_mv, high_eng) = window[1];
            let (gain, offset) = calibration::two_point_calibration(
                low_mv, high_mv, low_eng, high_eng
            ).unwrap_or((1.0, 0.0));

            let base_idx = (seg_idx as u8) * 3;

            // valid_from
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_lin_index(), data: codec::encode_u8(base_idx),
            });
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_coeff_at_idx(), data: codec::encode_f32_be(low_mv),
            });
            // gain
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_lin_index(), data: codec::encode_u8(base_idx + 1),
            });
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_coeff_at_idx(), data: codec::encode_f32_be(gain),
            });
            // offset
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_lin_index(), data: codec::encode_u8(base_idx + 2),
            });
            send_tracked(state, ble, BleCommand::WriteCharacteristic {
                uuid: uuids::char_coeff_at_idx(), data: codec::encode_f32_be(offset),
            });
        }

        // Final valid_to
        let last_mv = cal_points.last().unwrap().0;
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_lin_index(), data: codec::encode_u8((num_segments as u8) * 3),
        });
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_coeff_at_idx(), data: codec::encode_f32_be(last_mv),
        });

        // Set lin_points
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_lin_points(), data: codec::encode_u8(num_segments as u8),
        });

        // Set data_gain = 1.0, data_offset = 0.0
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_data_gain(), data: codec::encode_f32_be(1.0),
        });
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_data_offset(), data: codec::encode_f32_be(0.0),
        });

        // Trigger CalculateCoefficients
        ble.send(BleCommand::ExecuteAction(DeviceAction::CalculateCoefficients));

        // Also populate the UI table cal with the imported points
        state.ui.table_cal.rows = cal_points.iter().map(|(mv, eng)| {
            TableCalRow {
                mv_per_v: format!("{mv}"),
                eng_value: format!("{eng}"),
            }
        }).collect();
        state.ui.table_cal.applied = true;
    }

    // Re-read calibration registers
    send_tracked(state, ble, BleCommand::ReadAll(uuids::all_calibration_uuids()));
}

// -- Table Calibration Sub-tab ----------------------------------------------

fn show_table_cal(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);

        // Live value banner (full width)
        show_live_value_banner(ui, state);

        ui.add_space(4.0);

        // Fetch Existing Calibration button
        show_fetch_calibration_bar(ui, state, ble, true);

        ui.add_space(8.0);

        // Pre-compute preview data before splitting columns (to avoid borrow issues)
        let parsed: Vec<Option<(f32, f32)>> = state.ui.table_cal.rows.iter().map(|r| {
            let mv: Option<f32> = r.mv_per_v.parse().ok();
            let eng: Option<f32> = r.eng_value.parse().ok();
            mv.zip(eng)
        }).collect();

        let all_valid = parsed.iter().all(|p| p.is_some()) && parsed.len() >= 2;
        let points: Vec<(f32, f32)> = parsed.iter().filter_map(|p| *p).collect();
        let ascending = points.windows(2).all(|w| w[1].0 > w[0].0);

        let segments: Vec<(f32, f32, f32, f32)> = if all_valid && ascending && points.len() >= 2 {
            points.windows(2).map(|w| {
                let (low_mv, low_eng) = w[0];
                let (high_mv, high_eng) = w[1];
                let (gain, offset) = calibration::two_point_calibration(
                    low_mv, high_mv, low_eng, high_eng
                ).unwrap_or((1.0, 0.0));
                (low_mv, gain, offset, high_mv)
            }).collect()
        } else {
            Vec::new()
        };

        let has_any_input = state.ui.table_cal.rows.iter().any(|r| !r.mv_per_v.is_empty() || !r.eng_value.is_empty());

        // Two-column layout: left = input table, right = preview
        ui.columns(2, |cols| {
            // LEFT: Input table
            widgets::section_header(&mut cols[0], "Table Calibration");

            cols[0].label(
                egui::RichText::new(
                    "Enter mV/V and engineering values from the sensor's\n\
                     calibration certificate. Minimum 2 rows required.\n\
                     Rows must be in ascending mV/V order."
                ).size(14.0)
            );
            cols[0].add_space(12.0);

            // Editable table
            let mut remove_idx = None;
            let row_count = state.ui.table_cal.rows.len();
            egui::Grid::new("table_cal_grid")
                .num_columns(4)
                .spacing([12.0, 6.0])
                .min_col_width(40.0)
                .show(&mut cols[0], |ui| {
                    ui.strong("Point");
                    ui.horizontal(|ui| {
                        ui.strong("mV/V (raw)");
                        widgets::help_icon(ui, "Raw sensor signal in millivolts per volt.\nTypically from the sensor datasheet or\ncaptured using Auto Calibration.");
                    });
                    ui.horizontal(|ui| {
                        ui.strong("Engineering Value");
                        widgets::help_icon(ui, "The desired output value in engineering units\n(e.g. kg, N m, lbf) at this mV/V point.");
                    });
                    ui.strong("");
                    ui.end_row();

                    for (i, row) in state.ui.table_cal.rows.iter_mut().enumerate() {
                        ui.label(egui::RichText::new(format!("{}", i + 1)).size(15.0));
                        ui.add_sized([140.0, 28.0],
                            egui::TextEdit::singleline(&mut row.mv_per_v)
                                .hint_text("e.g. 0.0")
                                .vertical_align(egui::Align::Center)
                        );
                        ui.add_sized([140.0, 28.0],
                            egui::TextEdit::singleline(&mut row.eng_value)
                                .hint_text("e.g. 0.0")
                                .vertical_align(egui::Align::Center)
                        );
                        if row_count > 2 {
                            if ui.small_button("X").clicked() {
                                remove_idx = Some(i);
                            }
                        } else {
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });

            if let Some(idx) = remove_idx {
                state.ui.table_cal.rows.remove(idx);
            }

            cols[0].add_space(8.0);

            cols[0].horizontal(|ui| {
                if ui.button("+ Add Row").clicked() && state.ui.table_cal.rows.len() < 16 {
                    state.ui.table_cal.rows.push(TableCalRow::default());
                }
                ui.add_space(8.0);
                let dark = ui.visuals().dark_mode;
                ui.label(
                    egui::RichText::new(format!("{} / 16 points", state.ui.table_cal.rows.len()))
                        .size(13.0).color(widgets::muted_text(dark))
                );
            });

            if all_valid && !ascending {
                cols[0].add_space(4.0);
                cols[0].label(
                    egui::RichText::new("mV/V values must be in strictly ascending order.")
                        .color(widgets::COLOR_ERROR)
                );
            }

            if !all_valid && has_any_input {
                cols[0].add_space(4.0);
                cols[0].label(
                    egui::RichText::new("Fill in all mV/V and engineering values (valid numbers required).")
                        .color(widgets::COLOR_WARNING)
                );
            }

            // Show success message
            if state.ui.table_cal.applied {
                cols[0].add_space(8.0);
                cols[0].label(
                    egui::RichText::new("Calibration written to device and Calculate Coefficients triggered.")
                        .size(15.0)
                        .color(widgets::COLOR_SUCCESS)
                );
            }

            // Show error
            if let Some(err) = &state.ui.table_cal.error.clone() {
                cols[0].add_space(8.0);
                cols[0].label(
                    egui::RichText::new(err)
                        .color(widgets::COLOR_ERROR)
                );
            }

            // RIGHT: Preview + Apply
            if !segments.is_empty() {
                widgets::section_header(&mut cols[1], "Preview");

                egui::Grid::new("table_cal_preview")
                    .num_columns(5)
                    .spacing([16.0, 4.0])
                    .striped(true)
                    .min_col_width(80.0)
                    .show(&mut cols[1], |ui| {
                        ui.strong("Segment");
                        ui.strong("Valid From");
                        ui.strong("Gain");
                        ui.strong("Offset");
                        ui.strong("Valid To");
                        ui.end_row();

                        for (i, (vf, g, o, vt)) in segments.iter().enumerate() {
                            ui.label(format!("{}", i + 1));
                            ui.monospace(format!("{vf:.6}"));
                            ui.monospace(format!("{g:.6}"));
                            ui.monospace(format!("{o:.6}"));
                            ui.monospace(format!("{vt:.6}"));
                            ui.end_row();
                        }
                    });

                cols[1].add_space(16.0);

                cols[1].horizontal(|ui| {
                    if ui.add_sized([220.0, 36.0], egui::Button::new(
                        egui::RichText::new("Apply to Device").size(16.0).color(egui::Color32::WHITE)
                    ).fill(widgets::COLOR_BTN_GREEN)).clicked() {
                        apply_table_calibration(state, ble, &points);
                    }

                    if ui.add_sized([100.0, 36.0], egui::Button::new("Clear")).clicked() {
                        state.ui.table_cal.rows = vec![TableCalRow::default(), TableCalRow::default()];
                        state.ui.table_cal.applied = false;
                        state.ui.table_cal.error = None;
                    }
                });
            }
        });
    });
}

fn apply_table_calibration(state: &mut AppState, ble: &BleHandle, points: &[(f32, f32)]) {
    let num_segments = points.len() - 1;

    // Set lin_repeat = 3 (valid_from, gain, offset per segment)
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_lin_repeat(),
        data: codec::encode_u8(3),
    });

    // Write each segment
    for (seg_idx, window) in points.windows(2).enumerate() {
        let (low_mv, low_eng) = window[0];
        let (high_mv, high_eng) = window[1];

        let (gain, offset) = calibration::two_point_calibration(
            low_mv, high_mv, low_eng, high_eng
        ).unwrap_or((1.0, 0.0));

        let base = (seg_idx as u8) * 3;

        // valid_from
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_lin_index(),
            data: codec::encode_u8(base),
        });
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_coeff_at_idx(),
            data: codec::encode_f32_be(low_mv),
        });

        // gain
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_lin_index(),
            data: codec::encode_u8(base + 1),
        });
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_coeff_at_idx(),
            data: codec::encode_f32_be(gain),
        });

        // offset
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_lin_index(),
            data: codec::encode_u8(base + 2),
        });
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_coeff_at_idx(),
            data: codec::encode_f32_be(offset),
        });
    }

    // Write final valid_to (= last point's mV/V)
    let last_mv = points.last().map(|p| p.0).unwrap_or(0.0);
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_lin_index(),
        data: codec::encode_u8((num_segments as u8) * 3),
    });
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_coeff_at_idx(),
        data: codec::encode_f32_be(last_mv),
    });

    // Set lin_points
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_lin_points(),
        data: codec::encode_u8(num_segments as u8),
    });

    // Set data_gain = 1.0, data_offset = 0.0
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_data_gain(),
        data: codec::encode_f32_be(1.0),
    });
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_data_offset(),
        data: codec::encode_f32_be(0.0),
    });

    // Trigger Calculate Coefficients
    ble.send(BleCommand::ExecuteAction(DeviceAction::CalculateCoefficients));

    state.ui.table_cal.applied = true;
    state.ui.table_cal.error = None;
}

// -- Advanced / Registers Sub-tab -------------------------------------------

/// Format an advanced param value using the correct data type
fn format_adv_value(state: &AppState, param: AdvancedParam) -> String {
    let index = param.index();
    match param.data_type() {
        ParamType::Float => {
            state.advanced.get_f32(index)
                .map(|v| {
                    if v == 0.0 || (v.abs() >= 0.001 && v.abs() < 1_000_000.0) {
                        format!("{v:.4}")
                    } else {
                        format!("{v:.4e}")
                    }
                })
                .unwrap_or_else(|| "--".to_string())
        }
        ParamType::Uint32 => {
            state.advanced.get_u32(index)
                .map(|v| format!("{v}"))
                .unwrap_or_else(|| "--".to_string())
        }
        ParamType::Uint8 => {
            state.advanced.get_u8(index)
                .map(|v| format!("{v}"))
                .unwrap_or_else(|| "--".to_string())
        }
    }
}

/// Write an advanced param using the correct encoding for its data type
fn write_adv_param(state: &mut AppState, ble: &BleHandle, param: AdvancedParam) {
    let index = param.index();
    let input = &state.ui.edit.adv_data;
    match param.data_type() {
        ParamType::Float => {
            if let Ok(val) = input.parse::<f32>() {
                send_tracked(state, ble, BleCommand::WriteAdvanced {
                    index,
                    data: val.to_be_bytes().to_vec(),
                });
            }
        }
        ParamType::Uint32 => {
            if let Ok(val) = input.parse::<u32>() {
                send_tracked(state, ble, BleCommand::WriteAdvanced {
                    index,
                    data: val.to_be_bytes().to_vec(),
                });
            }
        }
        ParamType::Uint8 => {
            if let Ok(val) = input.parse::<u8>() {
                send_tracked(state, ble, BleCommand::WriteAdvanced {
                    index,
                    data: vec![val],
                });
            }
        }
    }
}

fn show_advanced(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);

        // -- Row: Advanced Params (left) | Linearisation (right)
        ui.columns(2, |cols| {
            // LEFT: Advanced Parameters
            widgets::section_header(&mut cols[0], "Advanced Parameters");

            let dark = cols[0].visuals().dark_mode;
            cols[0].label(
                egui::RichText::new("Hover over parameter names for descriptions.")
                    .size(13.0).color(widgets::muted_text(dark))
            );
            cols[0].add_space(4.0);

            cols[0].horizontal(|ui| {
                if ui.button("Read All Advanced").clicked() {
                    for param in AdvancedParam::ALL {
                        send_tracked(state, ble, BleCommand::ReadAdvanced { index: param.index() });
                    }
                }
            });
            cols[0].add_space(4.0);

            egui::Grid::new("adv_params")
                .num_columns(4)
                .spacing([12.0, 6.0])
                .min_col_width(40.0)
                .show(&mut cols[0], |ui| {
                    ui.strong("Parameter");
                    ui.strong("Value");
                    ui.strong("Edit");
                    ui.strong("");
                    ui.end_row();

                    for param in AdvancedParam::ALL {
                        let index = param.index();
                        let is_readonly = *param == AdvancedParam::PeakValue
                            || *param == AdvancedParam::TroughValue;

                        // Parameter name with tooltip description
                        let label_resp = ui.add_sized([160.0, 26.0],
                            egui::Label::new(egui::RichText::new(param.label()).size(14.0))
                                .sense(egui::Sense::hover())
                        );
                        label_resp.on_hover_text_at_pointer(param.description());

                        // Value display - use correct data type
                        let value_str = format_adv_value(state, *param);
                        adv_value_or_spinner(ui, &value_str, index, state);

                        if is_readonly {
                            ui.label("");
                        } else {
                            ui.add_sized([100.0, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.adv_data).vertical_align(egui::Align::Center));
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Read").clicked() {
                                send_tracked(state, ble, BleCommand::ReadAdvanced { index });
                            }
                            if !is_readonly {
                                if ui.button("Write").clicked() {
                                    write_adv_param(state, ble, *param);
                                }
                            }
                        });
                        ui.end_row();
                    }
                });

            // RIGHT: Linearisation Table
            widgets::section_header(&mut cols[1], "Linearisation Table");

            cols[1].horizontal(|ui| {
                if ui.button("Refresh Table").clicked() {
                    send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_lin_points()));
                }
                ui.add_space(8.0);
                ui.label(format!(
                    "Points: {}",
                    state.calibration.lin_points.unwrap_or(0)
                ));
            });

            cols[1].add_space(4.0);

            // Linearisation controls
            egui::Grid::new("lin_controls")
                .num_columns(4)
                .spacing([12.0, 8.0])
                .show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Lin Index:");
                        widgets::help_icon(ui, "Index into the linearisation coefficient table.\nSet this before reading/writing individual coefficients.\nEach segment uses 3 consecutive indices.");
                    });
                    ui.add_sized([60.0, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.lin_index));
                    let li_str = state.calibration.lin_index
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &li_str, uuids::char_lin_index(), state);
                    ui.horizontal(|ui| {
                        if ui.button("Read").clicked() {
                            send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_lin_index()));
                        }
                        if ui.button("Write").clicked() {
                            if let Ok(v) = state.ui.edit.lin_index.parse::<u8>() {
                                send_tracked(state, ble, BleCommand::WriteCharacteristic {
                                    uuid: uuids::char_lin_index(), data: codec::encode_u8(v),
                                });
                            }
                        }
                    });
                    ui.end_row();

                    ui.horizontal(|ui| {
                        ui.label("Lin Repeat:");
                        widgets::help_icon(ui, "Number of coefficients per linearisation segment.\nMust be 3 (valid_from, gain, offset).\nDo not change unless you know what you're doing.");
                    });
                    ui.add_sized([60.0, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.lin_repeat));
                    let lr_str = state.calibration.lin_repeat
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &lr_str, uuids::char_lin_repeat(), state);
                    ui.horizontal(|ui| {
                        if ui.button("Read").clicked() {
                            send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_lin_repeat()));
                        }
                        if ui.button("Write").clicked() {
                            if let Ok(v) = state.ui.edit.lin_repeat.parse::<u8>() {
                                send_tracked(state, ble, BleCommand::WriteCharacteristic {
                                    uuid: uuids::char_lin_repeat(), data: codec::encode_u8(v),
                                });
                            }
                        }
                    });
                    ui.end_row();

                    ui.horizontal(|ui| {
                        ui.label("Lin Points:");
                        widgets::help_icon(ui, "Number of active linearisation segments (max 15).\nSet automatically when applying calibration.\nEach segment maps a range of mV/V to engineering values.");
                    });
                    ui.add_sized([60.0, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.lin_points));
                    let lp_str = state.calibration.lin_points
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "--".to_string());
                    value_or_spinner(ui, &lp_str, uuids::char_lin_points(), state);
                    ui.horizontal(|ui| {
                        if ui.button("Read").clicked() {
                            send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_lin_points()));
                        }
                        if ui.button("Write").clicked() {
                            if let Ok(v) = state.ui.edit.lin_points.parse::<u8>() {
                                send_tracked(state, ble, BleCommand::WriteCharacteristic {
                                    uuid: uuids::char_lin_points(), data: codec::encode_u8(v),
                                });
                            }
                        }
                    });
                    ui.end_row();
                });

            cols[1].add_space(8.0);

            // Linearisation data table
            egui::Grid::new("lin_table")
                .num_columns(5)
                .spacing([16.0, 4.0])
                .striped(true)
                .min_col_width(70.0)
                .show(&mut cols[1], |ui| {
                    ui.strong("Index");
                    ui.strong("Valid From");
                    ui.strong("Gain");
                    ui.strong("Offset");
                    ui.strong("Valid To");
                    ui.end_row();

                    if state.calibration.linearisation_table.is_empty() {
                        for i in 1..=15 {
                            ui.label(format!("{i}"));
                            ui.monospace("-");
                            ui.monospace("-");
                            ui.monospace("-");
                            ui.monospace("-");
                            ui.end_row();
                        }
                    } else {
                        for entry in &state.calibration.linearisation_table {
                            ui.label(format!("{}", entry.index));
                            ui.monospace(format!("{}", entry.valid_from));
                            ui.monospace(format!("{}", entry.gain));
                            ui.monospace(format!("{}", entry.offset));
                            ui.monospace(format!("{}", entry.valid_to));
                            ui.end_row();
                        }
                    }
                });
        });
    });
}
