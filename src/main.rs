#![allow(dead_code)]
#![windows_subsystem = "windows"]

mod protocol;
mod ble;
mod state;
mod app;
mod ui;

use eframe::egui;
use app::B24App;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let icon = eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
        .expect("Failed to load app icon");

    let options = eframe::NativeOptions {
        centered: true,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title(format!("B24 Tool v{VERSION}"))
            .with_min_inner_size([800.0, 600.0])
            .with_icon(icon),
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
