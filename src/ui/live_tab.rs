use eframe::egui;
use egui_plot::{Plot, Line, PlotPoints};
use crate::state::{AppState, ViewSource, Tab};
use crate::ble::manager::BleHandle;
use crate::protocol::types::DataUnits;
use crate::ui::widgets::{self, status_indicator};

pub fn show(ui: &mut egui::Ui, state: &mut AppState, _ble: &BleHandle) {
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        widgets::page_header(ui, "Live Data");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_sized([140.0, widgets::BTN_HEIGHT_HEADER], egui::Button::new(
                egui::RichText::new("Fullscreen View").size(15.0)
            ).fill(widgets::COLOR_BTN_GREEN)).clicked() {
                state.ui.view_mode.active = true;
                state.ui.view_mode.source = ViewSource::Connected;
                state.ui.view_mode.device_name = "Connected Device".to_string();
                state.ui.active_tab = Tab::Connect;
            }
        });
    });
    ui.separator();

    // Large current value display
    let dp = state.ui.view_mode.display_decimals;
    let value_text = state.live_data.current_value
        .map(|v| format!("{v:.dp$}"))
        .unwrap_or_else(|| "--".to_string());

    let units_text = state.calibration.cal_units
        .or(state.live_data.current_units)
        .map(|u| DataUnits::from_byte(u).label().to_string())
        .unwrap_or_default();

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::BOTTOM), |ui| {
            ui.label(
                egui::RichText::new(&value_text)
                    .size(32.0)
                    .monospace(),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(&units_text)
                    .size(16.0)
                    .color(widgets::bright_info(ui.visuals().dark_mode)),
            );
        });
    });

    ui.add_space(8.0);

    // Status flags
    if let Some(status) = state.live_data.current_status {
        ui.horizontal(|ui| {
            status_indicator(ui, "ShuntCal", status.shunt_cal);
            status_indicator(ui, "Integrity", status.integrity);
            status_indicator(ui, "Tare", status.not_gross);
            status_indicator(ui, "OverRange", status.over_range);
            status_indicator(ui, "Fast", status.fast_mode);
            status_indicator(ui, "BattLow", status.battery_low);
            status_indicator(ui, "DigIn", status.digital_input);
        });

        ui.horizontal(|ui| {
            let status_byte = status.to_byte();
            ui.label(format!("Status: {status_byte} (0x{status_byte:02X})"));
            ui.label(status.description());
        });
    }

    ui.add_space(8.0);

    // Time-series plot
    let points: PlotPoints = state
        .live_data
        .history
        .iter()
        .map(|(t, v)| [*t, *v as f64])
        .collect();

    let plot = Plot::new("live_plot")
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
                .color(egui::Color32::from_rgb(50, 150, 255)),
        );
    });

    ui.add_space(8.0);

    // Stats
    if !state.live_data.history.is_empty() {
        let values: Vec<f32> = state.live_data.history.iter().map(|(_, v)| *v).collect();
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let avg: f32 = values.iter().sum::<f32>() / values.len() as f32;

        ui.horizontal(|ui| {
            ui.label(format!("Points: {}", values.len()));
            ui.separator();
            ui.label(format!("Min: {min:.dp$}"));
            ui.separator();
            ui.label(format!("Max: {max:.dp$}"));
            ui.separator();
            ui.label(format!("Avg: {avg:.dp$}"));
        });

        if ui.button("Clear History").clicked() {
            state.live_data.history.clear();
            state.live_data.start_time = None;
        }
    }
}
