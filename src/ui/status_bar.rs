use eframe::egui;
use crate::state::{AppState, ConnectionPhase};
use crate::protocol::types::DataUnits;
use crate::ui::widgets;

pub fn show(ui: &mut egui::Ui, state: &AppState) {
    ui.horizontal(|ui| {
        // Connection status indicator
        let (color, text) = match state.connection.phase {
            ConnectionPhase::Disconnected => (widgets::COLOR_ERROR, "Disconnected"),
            ConnectionPhase::Scanning => (widgets::COLOR_WARNING, "Scanning..."),
            ConnectionPhase::Connecting => (widgets::COLOR_WARNING, "Connecting..."),
            ConnectionPhase::Connected => (widgets::COLOR_SUCCESS, "Connected"),
        };

        // Colored dot
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 5.0, color);
        ui.colored_label(color, text);

        ui.separator();

        // Connected device info
        if state.connection.phase == ConnectionPhase::Connected {
            if let Some(tag) = &state.config.data_tag {
                ui.label(format!("Tag: {tag}"));
                ui.separator();
            }
            if let Some(model) = &state.config.model_name {
                ui.label(model.as_str());
                ui.separator();
            }
        }

        // Battery
        if let Some(batt) = state.config.battery_value {
            let batt_color = if batt < 2.5 {
                widgets::COLOR_ERROR
            } else {
                widgets::COLOR_SUCCESS
            };
            ui.colored_label(batt_color, format!("Batt: {batt:.2}V"));
            ui.separator();
        }

        // Live value
        if let Some(val) = state.live_data.current_value {
            let units_label = state.live_data.current_units
                .map(|u| DataUnits::from_byte(u).label())
                .unwrap_or("");
            ui.label(format!("Value: {val:.4} {units_label}"));
            ui.separator();
        }

        // Status description
        if let Some(status) = state.live_data.current_status {
            let desc = status.description();
            let color = if desc == "OK" {
                widgets::COLOR_SUCCESS
            } else {
                widgets::COLOR_WARNING
            };
            ui.colored_label(color, desc);
        }

        // Error
        if let Some(err) = &state.connection.error_message {
            ui.separator();
            ui.colored_label(widgets::COLOR_ERROR, err.as_str());
        }
    });
}
