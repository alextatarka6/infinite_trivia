# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```zsh
# Source cargo if not in PATH (required once per shell session if cargo not found)
source "$HOME/.cargo/env"

# Build (dev)
cargo build

# Run
cargo run

# Release build
cargo build --release
```

There are no automated tests. Verify changes by running the app and exercising the relevant mode.

## Architecture

**Stack:** Rust, `egui 0.29` / `eframe 0.29` immediate-mode GUI. `tokio` + `reqwest` for async API calls. `serde_json` for Jeopardy data and future multiplayer serialization.

**Entry point:** `src/main.rs` → `eframe::run_native` → `app::App`. The `App::update()` is called every frame; `theme::apply(ctx)` is called at the top of every frame to enforce the Jeopardy-inspired blue/gold palette.

**Screen routing:** `game/state.rs` defines `enum Screen { Home, Jeopardy, Trivia, Quizbowl }`. `app.rs` matches on `self.screen` inside `update()` and delegates to the appropriate subsystem.

**Jeopardy mode — board/overlay separation:** The board (`jeopardy_board: Option<Board>`) is kept alive throughout the game. Clue, Daily Double, and Final Jeopardy screens are stored as `jeopardy_overlay: Option<JeopardyOverlay>` and swapped in/out. This preserves the `used[][]` tile state across clue visits. Score lives on the `Board` struct.

**Async pattern:** API calls (`opentdb::fetch_questions`, `qbreader::fetch_tossup`) are spawned onto `self.rt` (a `tokio::Runtime`) and write results into `Arc<Mutex<Option<Result<…>>>>` slots. `App::update()` polls these slots each frame via `try_lock()` and promotes the result into the relevant screen struct.

**Jeopardy data:** `assets/JEOPARDY_QUESTIONS1.json` (~55 MB, 216 k records) is loaded synchronously at startup into a `HashMap<(round, category), Vec<JeopardyRecord>>`. JSON fields are `category`, `air_date`, `question`, `value` (`"$200"`, `"$1,000"`, or `null`/`"None"`), `answer`, `round`, `show_number`. `random_board()` Fisher-Yates shuffles the full category list before selecting 6 to avoid alphabetically-adjacent results.

**Scoring (`game/scoring.rs`):**
- Jeopardy: clue face value, negated on wrong answer; Daily Double uses a player wager.
- Trivia: +10 per correct answer, streak multiplier kicks in every 3 in a row (max 4×).
- Quizbowl: correct early buzz = 20 pts; mid = 15; late = 10; wrong = −5.

**Theme (`src/theme.rs`):** All color constants and font helpers live here. Key colors: `BLUE_BG` (#030750 panel background), `BLUE_DARK` (#060CE9 tile/button fill), `GOLD` (#F5A623 text/accent). Font helpers: `heading_font()` 28 pt, `subheading_font()` 18 pt, `body_font()` 16 pt, `dollar_font()` 22 pt.

**Multiplayer scaffold (intentionally unused):** `game/session.rs` (`Player`, `Session`) and `game/state.rs` (`GameState` with `Serialize`/`Deserialize`) exist as extension points for a future Jackbox-style WebSocket layer. Dead-code warnings for these are expected and should not be removed.

**Platform notes:** `build.rs` embeds a Windows icon via `winres` (Windows-only build dependency). For macOS distribution use `cargo-bundle`. Release profile uses `lto = true`, `strip = true`.
