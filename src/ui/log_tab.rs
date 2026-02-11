use eframe::egui;
use crate::state::AppState;
use crate::ble::manager::BleHandle;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, _ble: &BleHandle) {
    ui.heading("Data Log");
    ui.separator();

    ui.horizontal(|ui| {
        let label = if state.log.is_logging {
            "Stop Logging"
        } else {
            "Start Logging"
        };
        if ui.button(label).clicked() {
            state.log.is_logging = !state.log.is_logging;
        }

        ui.separator();
        ui.label(format!("Entries: {}", state.log.entries.len()));

        if ui.button("Clear Log").clicked() {
            state.log.entries.clear();
        }

        if ui.button("Export CSV").clicked() {
            export_csv(state);
        }
    });

    if state.log.is_logging {
        ui.colored_label(egui::Color32::from_rgb(255, 80, 80), "Recording...");
    }

    ui.add_space(8.0);

    // Log table
    egui::ScrollArea::vertical()
        .max_height(500.0)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            egui::Grid::new("log_grid")
                .num_columns(4)
                .spacing([16.0, 2.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Timestamp");
                    ui.strong("Value");
                    ui.strong("Status");
                    ui.strong("Units");
                    ui.end_row();

                    // Show last 500 entries for performance
                    let start = state.log.entries.len().saturating_sub(500);
                    for entry in &state.log.entries[start..] {
                        ui.label(entry.timestamp.format("%H:%M:%S%.3f").to_string());
                        ui.monospace(format!("{:.6}", entry.value));
                        ui.monospace(format!("0x{:02X}", entry.status));
                        ui.label(&entry.units);
                        ui.end_row();
                    }
                });
        });
}

fn export_csv(state: &AppState) {
    if state.log.entries.is_empty() {
        return;
    }

    let file = rfd::FileDialog::new()
        .add_filter("CSV", &["csv"])
        .set_file_name("b24_log.csv")
        .save_file();

    if let Some(path) = file {
        let mut wtr = match csv::Writer::from_path(&path) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to create CSV writer: {e}");
                return;
            }
        };

        let _ = wtr.write_record(["Timestamp", "Value", "Status", "Units"]);
        for entry in &state.log.entries {
            let _ = wtr.write_record([
                entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
                format!("{}", entry.value),
                format!("{}", entry.status),
                entry.units.clone(),
            ]);
        }
        let _ = wtr.flush();
        log::info!("Exported {} entries to {}", state.log.entries.len(), path.display());
    }
}
