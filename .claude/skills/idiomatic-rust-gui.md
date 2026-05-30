# Name: Idiomatic Rust GUI
# Description: Enforces modern eframe & egui practices in Rust

## Core Constraints:
1. **Never use retained-mode logic:** Remember that egui is immediate-mode. UI state must live in a central struct (e.g., `MyApp`), and the `update` function draws widgets based on it every frame.
2. **Standard Library:** Use `eframe` as the application container.
3. **Control Flow:** Pass mutable references (e.g., `&mut self.field`) into widgets. Do not attempt to store persistent, detached component states.
4. **Error Handling:** Use standard `Result` and `eframe::Error` within `main()`. Do not unwrap recklessly.