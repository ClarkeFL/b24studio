#![allow(dead_code)]

mod protocol;
mod ble;
mod state;
mod app;
mod ui;

use eframe::egui;
use app::B24App;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("B24 Telemetry Tool")
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "b24-tool",
        options,
        Box::new(|cc| Ok(Box::new(B24App::new(cc)))),
    ) {
        eprintln!("Failed to start application: {e}");
    }
}
