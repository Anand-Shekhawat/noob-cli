//! Noob's own window. It only draws: everything Noob does happens in the shared
//! engine (`noob_cli`), exactly as it does in the terminal.
#![windows_subsystem = "windows"] // no extra black console window on Windows

mod view;

use eframe::egui;

/// The window's icon, built into the program.
const ICON: &[u8] = include_bytes!("../../assets/noob-icon-512.png");

fn main() -> eframe::Result {
    noob_cli::style::turn_off(); // the window draws its own colours
    // Opened from the Dock or Start menu, an app starts in a system folder. Your home
    // folder is a friendlier place to begin. (If that fails, it just stays put.)
    if let Some(home) = std::env::home_dir() {
        let _ = std::env::set_current_dir(home);
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Noob")
        .with_inner_size([860.0, 580.0])
        .with_min_inner_size([480.0, 320.0]);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        // Saving a picture of the window only works with the "glow" way of drawing.
        #[cfg(feature = "screenshot")]
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Noob",
        options,
        Box::new(|_| Ok(Box::new(view::NoobApp::new()))),
    )
}
