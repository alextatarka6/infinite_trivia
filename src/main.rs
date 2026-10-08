mod api;
mod app;
mod game;
mod multiplayer;
mod screens;
mod server;
mod theme;

fn main() -> eframe::Result<()> {
    // Debug-only: run just the phone server (no window) for the phone test harness.
    #[cfg(debug_assertions)]
    if let Ok(port) = std::env::var("IT_PHONE_TEST") {
        let port = port.parse().unwrap_or(7878);
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let server = server::GameServer::start(port, &rt).expect("phone test server");
        println!("phone test server on {}", server.url());
        loop {
            while server.pop_input().is_some() {}
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Infinite Trivia")
            .with_inner_size([1280.0, 768.0])
            .with_min_inner_size([900.0, 600.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
                    .expect("valid app icon"),
            ),
        ..Default::default()
    };

    eframe::run_native(
        "Infinite Trivia",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
