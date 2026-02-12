use eframe::egui;
use crate::state::{AppState, ConnectionPhase, MobileExportRow};
use crate::ble::manager::BleHandle;
use crate::protocol::types::DataUnits;
use crate::ui::widgets;
use qrcode::QrCode;
use qrcode::types::Color as QrColor;

/// Timeout options in seconds (displayed in dropdown)
const TIMEOUT_OPTIONS: &[(usize, &str)] = &[
    (5, "5 seconds"),
    (10, "10 seconds"),
    (12, "12 seconds"),
    (15, "15 seconds"),
    (20, "20 seconds"),
    (30, "30 seconds"),
    (60, "60 seconds"),
];

pub fn show(ui: &mut egui::Ui, state: &mut AppState, _ble: &BleHandle) {
    ui.add_space(8.0);

    // Auto-populate from connected device (once)
    if state.connection.phase == ConnectionPhase::Connected
        && !state.ui.mobile_export.populated_from_device
    {
        if let Some(tag) = &state.config.data_tag {
            let row = &mut state.ui.mobile_export.transmitters[0];
            if row.data_tag.is_empty() {
                row.data_tag = tag.clone();
            }
            if row.description.is_empty() {
                row.description = state.config.model_name
                    .as_deref()
                    .unwrap_or("Load Cell")
                    .to_string();
            }
            if row.unit_byte.is_none() {
                row.unit_byte = state.calibration.cal_units;
            }
        }
        if state.ui.mobile_export.view_pin == "0000" {
            if let Some(pin) = &state.config.view_pin {
                state.ui.mobile_export.view_pin = pin.clone();
            }
        }
        state.ui.mobile_export.populated_from_device = true;
    }

    // Header
    ui.horizontal(|ui| {
        widgets::page_header(ui, "Mobile App Export");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_sized([120.0, widgets::BTN_HEIGHT_HEADER], egui::Button::new(
                egui::RichText::new("Copy JSON").size(15.0)
            )).clicked() {
                let json = generate_json(state);
                ui.ctx().copy_text(json.clone());
                state.ui.mobile_export.last_json = json;
            }
            ui.add_space(4.0);
            if ui.add_sized([120.0, widgets::BTN_HEIGHT_HEADER], egui::Button::new(
                egui::RichText::new("Export File").size(15.0)
            ).fill(widgets::COLOR_BTN_GREEN)).clicked() {
                let json = generate_json(state);
                state.ui.mobile_export.last_json = json.clone();
                export_json_file(state, &json);
            }
        });
    });
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);

        // Two-column layout: forms on left, QR code on right
        let available_width = ui.available_width();
        let qr_col_width = 320.0_f32.min(available_width * 0.35);
        let form_col_width = available_width - qr_col_width - 16.0; // 16px gap

        ui.horizontal_top(|ui| {
            // ══════════════════════════════════════════════════
            // LEFT COLUMN: Project Settings + Transmitters
            // ══════════════════════════════════════════════════
            ui.vertical(|ui| {
                ui.set_max_width(form_col_width);

                // ── Project Settings ──────────────────────────────
                widgets::section_header(ui, "Project Settings");

                let input_w = 220.0_f32.min(form_col_width - 160.0);

                egui::Grid::new("mobile_project_settings")
                    .num_columns(2)
                    .spacing([24.0, 10.0])
                    .min_col_width(130.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Project Name").size(15.0));
                        ui.add_sized([input_w, 30.0],
                            egui::TextEdit::singleline(&mut state.ui.mobile_export.project_name)
                                .hint_text("My Project")
                                .font(egui::TextStyle::Body)
                                .vertical_align(egui::Align::Center));
                        ui.end_row();

                        ui.label(egui::RichText::new("View PIN").size(15.0));
                        ui.add_sized([input_w, 30.0],
                            egui::TextEdit::singleline(&mut state.ui.mobile_export.view_pin)
                                .hint_text("0000")
                                .font(egui::TextStyle::Body)
                                .vertical_align(egui::Align::Center));
                        ui.end_row();

                        ui.label(egui::RichText::new("Timeout").size(15.0));
                        let timeout_label = TIMEOUT_OPTIONS
                            .get(state.ui.mobile_export.timeout_index)
                            .map(|(_, l)| *l)
                            .unwrap_or("12 seconds");
                        egui::ComboBox::from_id_salt("mobile_timeout")
                            .selected_text(timeout_label)
                            .width(input_w)
                            .show_ui(ui, |ui| {
                                for (idx, (_, label)) in TIMEOUT_OPTIONS.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut state.ui.mobile_export.timeout_index,
                                        idx,
                                        *label,
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label(egui::RichText::new("Decimal Places").size(15.0));
                        egui::ComboBox::from_id_salt("mobile_decimal")
                            .selected_text(format!("{}", state.ui.mobile_export.decimal_places))
                            .width(input_w)
                            .show_ui(ui, |ui| {
                                for dp in 0..=6 {
                                    ui.selectable_value(
                                        &mut state.ui.mobile_export.decimal_places,
                                        dp,
                                        format!("{dp}"),
                                    );
                                }
                            });
                        ui.end_row();
                    });

                ui.add_space(16.0);

                // ── Transmitters ──────────────────────────────────
                widgets::section_header(ui, "Transmitters");

                ui.label(
                    egui::RichText::new(
                        "Each transmitter gets a Metric tile on the mobile dashboard."
                    ).size(14.0)
                );
                ui.add_space(8.0);

                let mut remove_idx = None;
                let row_count = state.ui.mobile_export.transmitters.len();

                egui::Grid::new("mobile_transmitters")
                    .num_columns(5)
                    .spacing([12.0, 6.0])
                    .striped(true)
                    .min_col_width(40.0)
                    .show(ui, |ui| {
                        ui.strong("#");
                        ui.strong("Data Tag");
                        ui.strong("Description");
                        ui.strong("Unit");
                        ui.strong("");
                        ui.end_row();

                        for (i, row) in state.ui.mobile_export.transmitters.iter_mut().enumerate() {
                            ui.label(egui::RichText::new(format!("{}", i + 1)).size(15.0));

                            ui.add_sized([100.0, 28.0],
                                egui::TextEdit::singleline(&mut row.data_tag)
                                    .hint_text("e.g. 7BE5")
                                    .vertical_align(egui::Align::Center)
                            );

                            ui.add_sized([160.0, 28.0],
                                egui::TextEdit::singleline(&mut row.description)
                                    .hint_text("e.g. Load Cell 1")
                                    .vertical_align(egui::Align::Center)
                            );

                            // Unit dropdown
                            let unit_label = row.unit_byte
                                .map(|b| {
                                    let du = DataUnits::from_byte(b);
                                    du.dropdown_label()
                                })
                                .unwrap_or_else(|| "Select unit...".to_string());
                            egui::ComboBox::from_id_salt(format!("mobile_unit_{i}"))
                                .selected_text(&unit_label)
                                .width(160.0)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(row.unit_byte.is_none(), "-- None --").clicked() {
                                        row.unit_byte = None;
                                    }
                                    for unit in DataUnits::ALL {
                                        let label = unit.dropdown_label();
                                        let byte_val = unit.to_byte();
                                        let is_selected = row.unit_byte == Some(byte_val);
                                        if ui.selectable_label(is_selected, &label).clicked() {
                                            row.unit_byte = Some(byte_val);
                                        }
                                    }
                                });

                            if row_count > 1 {
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
                    state.ui.mobile_export.transmitters.remove(idx);
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("+ Add Transmitter").clicked() && row_count < 20 {
                        state.ui.mobile_export.transmitters.push(MobileExportRow::default());
                    }
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("{} transmitter(s)", state.ui.mobile_export.transmitters.len()))
                            .size(13.0).color(egui::Color32::GRAY)
                    );
                });
            });

            ui.add_space(16.0);

            // ══════════════════════════════════════════════════
            // RIGHT COLUMN: QR Code
            // ══════════════════════════════════════════════════
            ui.vertical(|ui| {
                ui.set_max_width(qr_col_width);

                widgets::section_header(ui, "QR Code");

                ui.label(
                    egui::RichText::new(
                        "Scan to copy JSON to clipboard,\nthen paste in the B24 Toolkit app."
                    ).size(13.0)
                );
                ui.add_space(8.0);

                // Generate compact JSON for QR (no pretty-print to save bytes)
                let compact_json = generate_json_compact(state);
                let json_bytes = compact_json.len();

                if json_bytes > 2953 {
                    ui.colored_label(
                        widgets::COLOR_WARNING,
                        format!(
                            "JSON is {json_bytes} bytes — too large\nfor QR (max ~2953). Use Export File."
                        ),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(format!("{json_bytes} / 2953 bytes"))
                            .size(12.0)
                            .color(egui::Color32::GRAY),
                    );

                    // Generate QR code and render as texture
                    match QrCode::new(compact_json.as_bytes()) {
                        Ok(code) => {
                            let qr_width = code.width();
                            let colors = code.to_colors();
                            let scale: usize = 4;
                            let quiet: usize = 4;
                            let total = qr_width + quiet * 2;
                            let img_size = total * scale;

                            let mut pixels = vec![egui::Color32::WHITE; img_size * img_size];
                            for y in 0..qr_width {
                                for x in 0..qr_width {
                                    if colors[y * qr_width + x] == QrColor::Dark {
                                        let bx = (x + quiet) * scale;
                                        let by = (y + quiet) * scale;
                                        for dy in 0..scale {
                                            for dx in 0..scale {
                                                pixels[(by + dy) * img_size + (bx + dx)] = egui::Color32::BLACK;
                                            }
                                        }
                                    }
                                }
                            }

                            let image = egui::ColorImage {
                                size: [img_size, img_size],
                                pixels,
                            };

                            let texture = ui.ctx().load_texture(
                                "mobile_export_qr",
                                image,
                                egui::TextureOptions::NEAREST,
                            );

                            // Display — fit within column width
                            let display_size = (qr_col_width - 16.0).max(100.0);
                            let sized = egui::load::SizedTexture::new(
                                texture.id(),
                                egui::Vec2::new(display_size, display_size),
                            );
                            ui.add(egui::Image::new(sized));
                        }
                        Err(e) => {
                            ui.colored_label(
                                widgets::COLOR_ERROR,
                                format!("QR error: {e}"),
                            );
                        }
                    }
                }
            });
        });

        ui.add_space(16.0);

        // ── JSON Preview (full width, below both columns) ────
        widgets::section_header(ui, "JSON Preview");

        // Auto-generate preview (pretty-printed)
        let json = generate_json(state);
        state.ui.mobile_export.last_json = json.clone();

        egui::ScrollArea::vertical()
            .id_salt("json_preview_scroll")
            .max_height(300.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut json.as_str())
                        .desired_width(f32::INFINITY)
                        .desired_rows(12)
                        .font(egui::TextStyle::Monospace)
                );
            });

        ui.add_space(16.0);
    });
}

/// Build the core serde_json::Value for the project.
/// Shared by both pretty and compact generators.
fn build_project_json(state: &AppState) -> serde_json::Value {
    let me = &state.ui.mobile_export;
    let timeout = TIMEOUT_OPTIONS
        .get(me.timeout_index)
        .map(|(t, _)| *t)
        .unwrap_or(12);

    let format_str = match me.decimal_places {
        0 => "0",
        1 => "0.0",
        2 => "0.00",
        3 => "0.000",
        4 => "0.0000",
        5 => "0.00000",
        6 => "0.000000",
        _ => "0.00",
    };

    let mut transmitter_exports = Vec::new();
    let mut metric_exports = Vec::new();
    let mut expression_exports = Vec::new();

    for (i, row) in me.transmitters.iter().enumerate() {
        let tag = row.data_tag.trim().to_uppercase();
        if tag.is_empty() { continue; }

        let unit = row.unit_byte
            .map(DataUnits::from_byte)
            .unwrap_or(DataUnits::Undefined);

        let unit_obj = serde_json::json!({
            "name": unit.app_name(),
            "symbol": unit.label(),
            "group": unit.app_group(),
            "hexValue": format!("0x{:02X}", unit.to_byte()),
            "id": -1,
            "ratio": 1
        });

        let description = if row.description.trim().is_empty() {
            format!("TX {}", tag)
        } else {
            row.description.trim().to_string()
        };

        // transmitterExports entry
        transmitter_exports.push(serde_json::json!({
            "description": description,
            "unit": unit.app_name(),
            "dataTag": tag
        }));

        // metricExports entry
        let position = i + 1;
        metric_exports.push(serde_json::json!({
            "title": description,
            "dashboardPosition": position,
            "metric": "Actual Value",
            "action": "Zero tile only",
            "format": format_str,
            "expression": format!("#{tag}"),
            "sourceUnit": unit_obj,
            "displayUnit": unit_obj,
            "lowIndicatorColour": 4,
            "middleIndicatorColour": 7,
            "highIndicatorColour": 6,
            "lowToMiddleThreshold": f64::MAX,
            "middleToHighThreshold": f64::MAX
        }));

        // expressionElementExports entry
        expression_exports.push(serde_json::json!({
            "elementId": 0,
            "tilePosition": position,
            "order": 0,
            "parentOrder": -1,
            "type": "transmitter",
            "value": tag,
            "selected": false,
            "selectable": false
        }));
    }

    let project_name = if me.project_name.trim().is_empty() {
        "B24 Project"
    } else {
        me.project_name.trim()
    };

    serde_json::json!({
        "name": project_name,
        "icon": 0,
        "viewPin": me.view_pin,
        "timeout": timeout,
        "transmitterExports": transmitter_exports,
        "metricExports": metric_exports,
        "expressionElementExports": expression_exports,
        "indicatorExports": [],
        "gaugeExports": [],
        "tankExports": [],
        "chartExports": []
    })
}

/// Fix f64::MAX exponent format: serde_json outputs "e308" but the B24 app expects "e+308"
fn fix_exponent_format(json: String) -> String {
    // serde_json serialises f64::MAX as 1.7976931348623157e308
    // The B24 Toolkit app exports it as 1.7976931348623157e+308
    // Ensure the '+' sign is present in the exponent for compatibility
    json.replace("e308", "e+308")
}

fn generate_json(state: &AppState) -> String {
    let project = build_project_json(state);
    let json = serde_json::to_string_pretty(&project).unwrap_or_else(|_| "{}".to_string());
    fix_exponent_format(json)
}

/// Generate compact (single-line) JSON for QR code — saves ~40% vs pretty-printed.
fn generate_json_compact(state: &AppState) -> String {
    let project = build_project_json(state);
    let json = serde_json::to_string(&project).unwrap_or_else(|_| "{}".to_string());
    fix_exponent_format(json)
}

fn export_json_file(state: &AppState, json: &str) {
    let name = if state.ui.mobile_export.project_name.trim().is_empty() {
        "b24_project"
    } else {
        state.ui.mobile_export.project_name.trim()
    };
    let filename = format!("{}.json", name.replace(' ', "_"));

    let task = rfd::FileDialog::new()
        .set_title("Export B24 Mobile App Project")
        .add_filter("JSON", &["json"])
        .set_file_name(&filename)
        .save_file();

    if let Some(path) = task {
        let _ = std::fs::write(path, json);
    }
}
