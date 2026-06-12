mod api;
mod app;
mod game;
mod screens;
mod theme;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Infinite Trivia")
            .with_inner_size([1280.0, 768.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Infinite Trivia",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
