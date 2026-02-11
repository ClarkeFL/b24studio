use eframe::egui;
use uuid::Uuid;
use crate::state::{AppState, CalSubTab, WizardPhase};
use crate::ble::commands::BleCommand;
use crate::ble::manager::BleHandle;
use crate::protocol::{uuids, codec};
use crate::protocol::types::{DeviceAction, AdvancedParam};
use crate::protocol::calibration;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    ui.add_space(4.0);

    // Sub-tab bar
    ui.horizontal(|ui| {
        ui.heading("Calibration");
        ui.add_space(16.0);
        ui.selectable_value(&mut state.ui.cal_sub_tab, CalSubTab::AutoCal, "Auto Calibration");
        ui.selectable_value(&mut state.ui.cal_sub_tab, CalSubTab::ManualCal, "Manual Calibration");
    });
    ui.separator();

    match state.ui.cal_sub_tab {
        CalSubTab::AutoCal => show_auto_cal(ui, state, ble),
        CalSubTab::ManualCal => show_manual_cal(ui, state, ble),
    }
}

// -- Helpers ----------------------------------------------------------------

fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(title)
            .size(17.0)
            .strong()
            .color(egui::Color32::from_rgb(130, 170, 220)),
    );
    ui.separator();
    ui.add_space(4.0);
}

fn send_tracked(state: &mut AppState, ble: &BleHandle, cmd: BleCommand) {
    state.track_send(&cmd);
    ble.send(cmd);
}

fn value_or_spinner(ui: &mut egui::Ui, text: &str, uuid: Uuid, state: &AppState) {
    if state.is_pending(&uuid) {
        ui.add(egui::Spinner::new().size(14.0));
    } else {
        ui.monospace(text);
    }
}

fn adv_value_or_spinner(ui: &mut egui::Ui, text: &str, index: u8, state: &AppState) {
    if state.is_adv_pending(index) {
        ui.add(egui::Spinner::new().size(14.0));
    } else {
        ui.monospace(text);
    }
}

// -- Auto Calibration Wizard ------------------------------------------------

fn show_auto_cal(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);

        match state.ui.cal_wizard.phase {
            WizardPhase::Setup => show_wizard_setup(ui, state),
            WizardPhase::CaptureZero => show_wizard_capture_zero(ui, state, ble),
            WizardPhase::CapturePoint => show_wizard_capture_point(ui, state, ble),
            WizardPhase::Complete => show_wizard_complete(ui, state),
        }

        // Results table (shown during CapturePoint and Complete phases)
        if !state.ui.cal_wizard.results.is_empty() {
            ui.add_space(16.0);
            section_header(ui, "Calibration Results");
            show_results_table(ui, state);
        }
    });
}

// -- Phase: Setup -----------------------------------------------------------

fn show_wizard_setup(ui: &mut egui::Ui, state: &mut AppState) {
    section_header(ui, "Auto Calibration Wizard");

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "This wizard guides you through calibrating the B24 sensor step by step.\n\
             It will capture a zero/reference reading, then each calibration point in sequence,\n\
             and write the linearisation coefficients to the device."
        ).size(14.0)
    );
    ui.add_space(12.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Number of calibration points:").size(15.0));
        ui.add_space(8.0);
        ui.add_sized([60.0, 30.0], egui::TextEdit::singleline(&mut state.ui.cal_wizard.total_points)
            .hint_text("1"));
        ui.add_space(8.0);
        ui.label(egui::RichText::new("(1 = simple two-point, up to 15 for multipoint)").size(13.0)
            .color(egui::Color32::GRAY));
    });

    ui.add_space(16.0);

    let total: u32 = state.ui.cal_wizard.total_points.parse().unwrap_or(0);
    let valid = total >= 1 && total <= 15;

    ui.horizontal(|ui| {
        let btn = ui.add_sized([180.0, 36.0], egui::Button::new(
            egui::RichText::new("Start Calibration").size(16.0)
        ).fill(egui::Color32::from_rgb(40, 120, 60)));
        if btn.clicked() && valid {
            state.ui.cal_wizard.phase = WizardPhase::CaptureZero;
            state.ui.cal_wizard.current_step = 0;
            state.ui.cal_wizard.known_value.clear();
            state.ui.cal_wizard.acquired_base = None;
            state.ui.cal_wizard.waiting_for_acquire = false;
            state.ui.cal_wizard.prev_known = 0.0;
            state.ui.cal_wizard.prev_base = 0.0;
            state.ui.cal_wizard.results.clear();
            state.ui.cal_wizard.pending_gain = None;
            state.ui.cal_wizard.pending_offset = None;
        }

        if !valid && !state.ui.cal_wizard.total_points.is_empty() {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Enter a value between 1 and 15")
                .color(egui::Color32::from_rgb(255, 100, 100)));
        }
    });
}

// -- Phase: CaptureZero (Step 0) -------------------------------------------

fn show_wizard_capture_zero(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    let total: u32 = state.ui.cal_wizard.total_points.parse().unwrap_or(1);

    section_header(ui, &format!(
        "Step 1 of {}: Capture Zero Reference",
        total + 1
    ));

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "Apply zero load (or your known reference value) to the sensor,\n\
             then enter the engineering value and click Acquire."
        ).size(14.0)
    );
    ui.add_space(12.0);

    egui::Grid::new("wizard_zero_grid")
        .num_columns(3)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Known value:").size(15.0));
            ui.add_sized([150.0, 30.0], egui::TextEdit::singleline(&mut state.ui.cal_wizard.known_value)
                .hint_text("0.0"));

            let acquire_enabled = !state.ui.cal_wizard.waiting_for_acquire;
            ui.add_enabled_ui(acquire_enabled, |ui| {
                if ui.add_sized([120.0, 30.0], egui::Button::new(
                    egui::RichText::new("Acquire").size(15.0)
                ).fill(egui::Color32::from_rgb(50, 100, 180))).clicked() {
                    state.ui.cal_wizard.waiting_for_acquire = true;
                    state.ui.cal_wizard.acquired_base = None;
                    send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_base_value()));
                }
            });
            ui.end_row();

            ui.label(egui::RichText::new("Raw base value:").size(15.0));
            if state.ui.cal_wizard.waiting_for_acquire {
                ui.add(egui::Spinner::new().size(16.0));
            } else if let Some(base) = state.ui.cal_wizard.acquired_base {
                ui.label(egui::RichText::new(format!("{base:.6}")).size(15.0).monospace()
                    .color(egui::Color32::from_rgb(80, 200, 80)));
            } else {
                ui.label(egui::RichText::new("--").size(15.0).monospace());
            }
            ui.label(""); // spacer
            ui.end_row();
        });

    ui.add_space(16.0);

    let can_proceed = state.ui.cal_wizard.acquired_base.is_some()
        && !state.ui.cal_wizard.waiting_for_acquire;

    ui.horizontal(|ui| {
        // Cancel
        if ui.add_sized([100.0, 32.0], egui::Button::new("Cancel")).clicked() {
            reset_wizard(state);
        }

        ui.add_space(16.0);

        // Next
        ui.add_enabled_ui(can_proceed, |ui| {
            if ui.add_sized([140.0, 32.0], egui::Button::new(
                egui::RichText::new("Next \u{2192}").size(15.0)
            ).fill(egui::Color32::from_rgb(40, 120, 60))).clicked() {
                let known: f32 = state.ui.cal_wizard.known_value.parse().unwrap_or(0.0);
                let base = state.ui.cal_wizard.acquired_base.unwrap_or(0.0);
                state.ui.cal_wizard.prev_known = known;
                state.ui.cal_wizard.prev_base = base;
                state.ui.cal_wizard.current_step = 1;
                state.ui.cal_wizard.known_value.clear();
                state.ui.cal_wizard.acquired_base = None;
                state.ui.cal_wizard.pending_gain = None;
                state.ui.cal_wizard.pending_offset = None;
                state.ui.cal_wizard.phase = WizardPhase::CapturePoint;
            }
        });
    });
}

// -- Phase: CapturePoint (Steps 1..N) --------------------------------------

fn show_wizard_capture_point(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    let total: u32 = state.ui.cal_wizard.total_points.parse().unwrap_or(1);
    let step = state.ui.cal_wizard.current_step;

    section_header(ui, &format!(
        "Step {} of {}: Calibration Point {}",
        step + 1,
        total + 1,
        step
    ));

    ui.add_space(4.0);

    // Show previous reference
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Previous reference:").size(14.0)
            .color(egui::Color32::GRAY));
        ui.label(egui::RichText::new(format!(
            "{:.6} eng @ raw {:.6}",
            state.ui.cal_wizard.prev_known,
            state.ui.cal_wizard.prev_base
        )).size(14.0).monospace().color(egui::Color32::GRAY));
    });

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "Apply the calibration load, enter the known engineering value, and click Acquire."
        ).size(14.0)
    );
    ui.add_space(12.0);

    egui::Grid::new("wizard_point_grid")
        .num_columns(3)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Known value:").size(15.0));
            ui.add_sized([150.0, 30.0], egui::TextEdit::singleline(&mut state.ui.cal_wizard.known_value)
                .hint_text("e.g. 100.0"));

            let acquire_enabled = !state.ui.cal_wizard.waiting_for_acquire;
            ui.add_enabled_ui(acquire_enabled, |ui| {
                if ui.add_sized([120.0, 30.0], egui::Button::new(
                    egui::RichText::new("Acquire").size(15.0)
                ).fill(egui::Color32::from_rgb(50, 100, 180))).clicked() {
                    state.ui.cal_wizard.waiting_for_acquire = true;
                    state.ui.cal_wizard.acquired_base = None;
                    send_tracked(state, ble, BleCommand::ReadCharacteristic(uuids::char_base_value()));
                }
            });
            ui.end_row();

            ui.label(egui::RichText::new("Raw base value:").size(15.0));
            if state.ui.cal_wizard.waiting_for_acquire {
                ui.add(egui::Spinner::new().size(16.0));
            } else if let Some(base) = state.ui.cal_wizard.acquired_base {
                ui.label(egui::RichText::new(format!("{base:.6}")).size(15.0).monospace()
                    .color(egui::Color32::from_rgb(80, 200, 80)));
            } else {
                ui.label(egui::RichText::new("--").size(15.0).monospace());
            }
            ui.label(""); // spacer
            ui.end_row();
        });

    // Auto-compute gain/offset when both values are available
    if let Some(high_base) = state.ui.cal_wizard.acquired_base {
        if !state.ui.cal_wizard.waiting_for_acquire {
            let high_target: f32 = state.ui.cal_wizard.known_value.parse().unwrap_or(0.0);
            let low_base = state.ui.cal_wizard.prev_base;
            let low_target = state.ui.cal_wizard.prev_known;

            if let Some((gain, offset)) = calibration::two_point_calibration(
                low_base, high_base, low_target, high_target
            ) {
                state.ui.cal_wizard.pending_gain = Some(gain);
                state.ui.cal_wizard.pending_offset = Some(offset);

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Gain: {gain:.6}"))
                        .monospace().size(15.0).color(egui::Color32::from_rgb(100, 200, 100)));
                    ui.add_space(24.0);
                    ui.label(egui::RichText::new(format!("Offset: {offset:.6}"))
                        .monospace().size(15.0).color(egui::Color32::from_rgb(100, 200, 100)));
                });
            }
        }
    }

    ui.add_space(16.0);

    let can_apply = state.ui.cal_wizard.pending_gain.is_some()
        && state.ui.cal_wizard.acquired_base.is_some()
        && !state.ui.cal_wizard.waiting_for_acquire;

    ui.horizontal(|ui| {
        // Cancel
        if ui.add_sized([100.0, 32.0], egui::Button::new("Cancel")).clicked() {
            reset_wizard(state);
        }

        ui.add_space(16.0);

        // Apply & Continue
        ui.add_enabled_ui(can_apply, |ui| {
            let btn_text = if step < total {
                "Apply & Continue \u{2192}"
            } else {
                "Apply & Finish \u{2713}"
            };

            if ui.add_sized([200.0, 36.0], egui::Button::new(
                egui::RichText::new(btn_text).size(15.0)
            ).fill(egui::Color32::from_rgb(40, 120, 60))).clicked() {
                apply_calibration_point(state, ble);
            }
        });
    });
}

/// Write linearisation coefficients to device and advance the wizard
fn apply_calibration_point(state: &mut AppState, ble: &BleHandle) {
    let step = state.ui.cal_wizard.current_step;
    let total: u32 = state.ui.cal_wizard.total_points.parse().unwrap_or(1);
    let gain = state.ui.cal_wizard.pending_gain.unwrap_or(1.0);
    let offset = state.ui.cal_wizard.pending_offset.unwrap_or(0.0);
    let low_base = state.ui.cal_wizard.prev_base;
    let high_base = state.ui.cal_wizard.acquired_base.unwrap_or(0.0);
    let low_target = state.ui.cal_wizard.prev_known;
    let high_target: f32 = state.ui.cal_wizard.known_value.parse().unwrap_or(0.0);

    let seg_idx = (step - 1) as u8; // 0-based segment index

    // On the first point, set lin_repeat = 3 (3 columns: valid_from, gain, offset)
    if step == 1 {
        send_tracked(state, ble, BleCommand::WriteCharacteristic {
            uuid: uuids::char_lin_repeat(),
            data: codec::encode_u8(3),
        });
    }

    // Write linearisation coefficients:
    // Set lin_index = seg_idx * 3, write coeff = valid_from (prev_base)
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_lin_index(),
        data: codec::encode_u8(seg_idx * 3),
    });
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_coeff_at_idx(),
        data: codec::encode_f32_be(low_base),
    });

    // Set lin_index = seg_idx * 3 + 1, write coeff = gain
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_lin_index(),
        data: codec::encode_u8(seg_idx * 3 + 1),
    });
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_coeff_at_idx(),
        data: codec::encode_f32_be(gain),
    });

    // Set lin_index = seg_idx * 3 + 2, write coeff = offset
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_lin_index(),
        data: codec::encode_u8(seg_idx * 3 + 2),
    });
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_coeff_at_idx(),
        data: codec::encode_f32_be(offset),
    });

    // Write valid_to for this segment (= high_base, also the next segment's valid_from)
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_lin_index(),
        data: codec::encode_u8(seg_idx * 3 + 3),
    });
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_coeff_at_idx(),
        data: codec::encode_f32_be(high_base),
    });

    // Set lin_points = step (number of rows so far)
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_lin_points(),
        data: codec::encode_u8(step as u8),
    });

    // Set data_gain = 1.0, data_offset = 0.0 (no unit conversion interference)
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_data_gain(),
        data: codec::encode_f32_be(1.0),
    });
    send_tracked(state, ble, BleCommand::WriteCharacteristic {
        uuid: uuids::char_data_offset(),
        data: codec::encode_f32_be(0.0),
    });

    // Trigger Calculate Coefficients action (index 38)
    ble.send(BleCommand::ExecuteAction(DeviceAction::CalculateCoefficients));

    // Add entry to results
    state.ui.cal_wizard.results.push(crate::state::WizardCalEntry {
        index: seg_idx + 1,
        valid_from: low_base,
        gain,
        offset,
        valid_to: high_base,
        target_from: low_target,
        target_to: high_target,
    });

    // Advance or complete
    if step < total {
        // Carry forward: current high becomes next low
        state.ui.cal_wizard.prev_known = high_target;
        state.ui.cal_wizard.prev_base = high_base;
        state.ui.cal_wizard.current_step = step + 1;
        state.ui.cal_wizard.known_value.clear();
        state.ui.cal_wizard.acquired_base = None;
        state.ui.cal_wizard.pending_gain = None;
        state.ui.cal_wizard.pending_offset = None;
    } else {
        state.ui.cal_wizard.phase = WizardPhase::Complete;
    }
}

// -- Phase: Complete --------------------------------------------------------

fn show_wizard_complete(ui: &mut egui::Ui, state: &mut AppState) {
    section_header(ui, "Calibration Complete");

    let count = state.ui.cal_wizard.results.len();
    ui.label(
        egui::RichText::new(format!(
            "\u{2713} Calibration complete! {} point(s) captured and written to device.",
            count
        ))
        .size(16.0)
        .color(egui::Color32::from_rgb(80, 200, 80)),
    );

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "The linearisation coefficients have been written and Calculate Coefficients triggered.\n\
             You can verify the results in the Manual Calibration tab."
        ).size(14.0).color(egui::Color32::GRAY)
    );

    ui.add_space(16.0);

    if ui.add_sized([200.0, 36.0], egui::Button::new(
        egui::RichText::new("Start New Calibration").size(15.0)
    )).clicked() {
        reset_wizard(state);
    }
}

// -- Results table ----------------------------------------------------------

fn show_results_table(ui: &mut egui::Ui, state: &AppState) {
    egui::Grid::new("wizard_results_table")
        .num_columns(7)
        .spacing([16.0, 4.0])
        .striped(true)
        .min_col_width(70.0)
        .show(ui, |ui| {
            ui.strong("Segment");
            ui.strong("Valid From");
            ui.strong("Gain");
            ui.strong("Offset");
            ui.strong("Valid To");
            ui.strong("Eng From");
            ui.strong("Eng To");
            ui.end_row();

            for entry in &state.ui.cal_wizard.results {
                ui.label(format!("{}", entry.index));
                ui.monospace(format!("{:.6}", entry.valid_from));
                ui.monospace(format!("{:.6}", entry.gain));
                ui.monospace(format!("{:.6}", entry.offset));
                ui.monospace(format!("{:.6}", entry.valid_to));
                ui.monospace(format!("{:.4}", entry.target_from));
                ui.monospace(format!("{:.4}", entry.target_to));
                ui.end_row();
            }
        });
}

// -- Reset wizard -----------------------------------------------------------

fn reset_wizard(state: &mut AppState) {
    state.ui.cal_wizard.phase = WizardPhase::Setup;
    state.ui.cal_wizard.total_points = "1".to_string();
    state.ui.cal_wizard.current_step = 0;
    state.ui.cal_wizard.known_value.clear();
    state.ui.cal_wizard.acquired_base = None;
    state.ui.cal_wizard.waiting_for_acquire = false;
    state.ui.cal_wizard.prev_known = 0.0;
    state.ui.cal_wizard.prev_base = 0.0;
    state.ui.cal_wizard.results.clear();
    state.ui.cal_wizard.pending_gain = None;
    state.ui.cal_wizard.pending_offset = None;
}

// -- Manual Calibration Sub-tab (Advanced & Linearisation) ------------------

fn show_manual_cal(ui: &mut egui::Ui, state: &mut AppState, ble: &BleHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);

        // -- Row: Advanced Params (left) | Linearisation (right)
        ui.columns(2, |cols| {
            // LEFT: Advanced Parameters
            section_header(&mut cols[0], "Advanced Parameters");

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

                        ui.add_sized([160.0, 26.0], egui::Label::new(param.label()));

                        let value_str = state.advanced.get_f32(index)
                            .map(|v| format!("{v}"))
                            .or_else(|| state.advanced.get_u8(index).map(|v| format!("{v}")))
                            .unwrap_or_else(|| "--".to_string());
                        adv_value_or_spinner(ui, &value_str, index, state);

                        if is_readonly {
                            ui.label("");
                        } else {
                            ui.add_sized([100.0, 30.0], egui::TextEdit::singleline(&mut state.ui.edit.adv_data));
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Read").clicked() {
                                send_tracked(state, ble, BleCommand::ReadAdvanced { index });
                            }
                            if !is_readonly {
                                if ui.button("Write").clicked() {
                                    if let Ok(val) = state.ui.edit.adv_data.parse::<f32>() {
                                        send_tracked(state, ble, BleCommand::WriteAdvanced {
                                            index,
                                            data: val.to_be_bytes().to_vec(),
                                        });
                                    } else if let Ok(val) = state.ui.edit.adv_data.parse::<u8>() {
                                        send_tracked(state, ble, BleCommand::WriteAdvanced {
                                            index,
                                            data: vec![val],
                                        });
                                    }
                                }
                            }
                        });
                        ui.end_row();
                    }
                });

            // RIGHT: Linearisation Table
            section_header(&mut cols[1], "Linearisation Table");

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
                    ui.label("Lin Index:");
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

                    ui.label("Lin Repeat:");
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

                    ui.label("Lin Points:");
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
