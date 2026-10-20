//! BW Darkroom: browse a folder, preview black-and-white styles on an image, and
//! export the chosen film → developer → print workflow at full resolution.
//!
//! The app is a thin shell over the engine. All photographic work happens in the
//! engine crates; rendering runs on background threads so the UI stays responsive.

mod app;
mod browser;
mod styles;
mod worker;
mod workflow;

fn main() -> eframe::Result {
    // Optional first argument: a folder to open, or an image to open and select.
    let start = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .filter(|p| p.exists());
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1480.0, 940.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("BW Darkroom"),
        ..Default::default()
    };
    eframe::run_native(
        "BW Darkroom",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc, start)))),
    )
}
