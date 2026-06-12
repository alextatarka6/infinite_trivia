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

**Dev screen gallery:** `IT_DEV=1 cargo run` boots into a dev gallery (`src/screens/dev.rs`, `Screen::DevMenu`) with buttons that jump straight into any in-game screen/state using mock data — no API or play-through needed. `IT_DEV=1 IT_SCREEN=<key> cargo run` opens one screen directly; keys: `trivia`, `trivia-summary`, `trivia-loading`, `qb`/`qb-reading`/`qb-buzzed`/`qb-correct`/`qb-incorrect`/`qb-timeout`, `bonus`, `setup`/`setup-teams`, `board`, `clue`, `clue-result`, `dd`, `dd-result`, `final`, `final-result`. Use this to eyeball UI work instead of scripting clicks.

## Architecture

**Stack:** Rust, `egui 0.29` / `eframe 0.29` immediate-mode GUI. `tokio` + `reqwest` for async API calls. `serde_json` for Jeopardy data and future multiplayer serialization.

**Entry point:** `src/main.rs` → `eframe::run_native` → `app::App`. The `App::update()` is called every frame; `theme::apply(ctx)` is called at the top of every frame to enforce the Jeopardy-inspired blue/gold palette.

**Screen routing:** `game/state.rs` defines `enum Screen { Home, JeopardySetup, Jeopardy, Trivia, Quizbowl, DevMenu }`. `app.rs` matches on `self.screen` inside `update()` and delegates to the appropriate subsystem.

**Shared UI kit (`src/screens/kit.rs`):** egui ports of the prototype's reusable components, used by every in-game screen so the look stays consistent with `studio.css` and the design tokens: `ambient(ctx)` (breathing radial backdrop + accent glow + bottom vignette), `enter_offset` (the `.it-enter` slide-up), `primary_button`/`ghost_button` (`.it-btn` pills with hover lift), `score_chip` (with bump), `quit_button` (fixed top-right), `loader` (bouncing dots), `summary` + `confetti` (round-complete), and overlay helpers `overlay_verdict` / `answer_reveal` / `centered_question` / `input_row` / `centered_primary`. Screens are drawn painter-first (compute a centered content column, apply the enter offset, paint within it) — see `trivia/question.rs` as the canonical example. Animations are driven off `ui.input(|i| i.time)` with `ctx.request_repaint()`.

**Jeopardy mode — board/overlay separation:** The board (`jeopardy_board: Option<Board>`) is kept alive throughout the game. Clue, Daily Double, and Final Jeopardy screens are stored as `jeopardy_overlay: Option<JeopardyOverlay>` and swapped in/out. This preserves the `used[][]` tile state across clue visits. Score lives on the `Board` struct.

**Async pattern:** API calls (`opentdb::fetch_questions`, `qbreader::fetch_tossup`) are spawned onto `self.rt` (a `tokio::Runtime`) and write results into `Arc<Mutex<Option<Result<…>>>>` slots. `App::update()` polls these slots each frame via `try_lock()` and promotes the result into the relevant screen struct.

**Jeopardy data:** `assets/JEOPARDY_QUESTIONS1.json` (~55 MB, 216 k records) is loaded synchronously at startup into a `HashMap<(round, category), Vec<JeopardyRecord>>`. JSON fields are `category`, `air_date`, `question`, `value` (`"$200"`, `"$1,000"`, or `null`/`"None"`), `answer`, `round`, `show_number`. `random_board()` Fisher-Yates shuffles the full category list before selecting 6 to avoid alphabetically-adjacent results.

**Scoring (`game/scoring.rs`):**
- Jeopardy: clue face value, negated on wrong answer; Daily Double uses a player wager.
- Trivia: +10 per correct answer, streak multiplier kicks in every 3 in a row (max 4×).
- Quizbowl: correct early buzz = 20 pts; mid = 15; late = 10; wrong = −5. A correct tossup is followed by a **bonus round** (`screens/quizbowl/bonus.rs`, real qbreader `/api/random-bonus`): the player answers 3 parts solo, +10 each (the "other players steal on a miss" multiplayer rule is deferred to the future Jackbox layer). `app.rs` orchestrates it via `quizbowl_bonus: Option<BonusScreen>` + `pending_bonus`, swapping the bonus in for the tossup view until it returns `BonusAction::Done`, then loading the next tossup.

**Design tokens (`theme/tokens.toml` → `build.rs` → `src/theme.rs`):** The palette, radii, spacing, and type scale are defined once in `theme/tokens.toml` (the single source of truth, extracted from the React prototype). `build.rs` parses it and generates `$OUT_DIR/design_tokens.rs` with compile-time `pub const`s — hex → `Color32::from_rgb`, base+alpha → premultiplied `Color32`, scalars → `RADIUS_*` / `SPACE_*` / `TYPE_*`. `theme.rs` `include!`s those constants (in an `#[allow(dead_code)]` `tokens` module, re-exported) and builds the egui `Style`/`Visuals` from them. **To change any color/size, edit `tokens.toml` and rebuild — never hand-edit the generated constants or hardcode hex in screens.** Key names: `BLUE_BG` (panel bg), `BLUE_DARK` (tile fill), `GOLD` (accent), `WHITE`/`INK_DIM` (text). Font helpers: `spectral()`/`spectral_medium()`/`spectral_italic()` (serif display), `plex()`/`plex_medium()`/`plex_semibold()` (sans UI), `mono()`/`mono_medium()`/`mono_semibold()` (numerals/tags); role helpers `heading_font()`, `body_font()`, `dollar_font()` pull sizes from `TYPE_*`. Fonts (Spectral, IBM Plex Sans/Mono) are loaded from `assets/*.ttf` in `App::new`.

**Design prototype (`design/`, gitignored):** A clickable React/CSS prototype of all screens lives in `design/app/` (notably `studio.css`, the locked theme). It is the visual reference but is **not** committed — only the extracted `theme/tokens.toml` is. When matching a screen to the prototype, read the relevant `.jsx`/`.css` there.

**Porting React/CSS → egui conventions:** When translating prototype markup into egui:
- **flex column** (`flex-direction: column`) → `ui.vertical(...)`; **flex row** → `ui.horizontal(...)`.
- **CSS `gap`** → `ui.spacing_mut().item_spacing` (set before the children), not manual `add_space` between every element.
- **cards / panels** (bordered, rounded, padded boxes) → `egui::Frame` with `.inner_margin(...)`, `.rounding(RADIUS_CARD)`, `.stroke(...)`, `.fill(BLUE_MID)` — don't hand-paint rect + stroke unless you need per-frame animation.
- **CSS grid** (e.g. the 6×N Jeopardy board) has no egui equivalent → use `egui_extras::StripBuilder` (nested strips) or `egui::Grid`; reach for `StripBuilder` when cells must flex to fill space.
- **CSS transitions/animations**: `transform: translateY(-6px)` hover lifts, `breathe`, `pulse`, etc. → drive with `ctx.animate_bool`/manual `sin(ctx.input(time))` and **translate the draw rect**, calling `ctx.request_repaint()` to keep the clock alive. Flag these as deliberate — animated widgets must paint their own rect (can't use a static `Frame`), so prefer them only where the prototype actually animates.

**Multiplayer scaffold (intentionally unused):** `game/session.rs` (`Player`, `Session`) and `game/state.rs` (`GameState` with `Serialize`/`Deserialize`) exist as extension points for a future Jackbox-style WebSocket layer. Dead-code warnings for these are expected and should not be removed.

**Platform notes:** `build.rs` does two jobs: generates the design-token constants (all platforms; reruns when `theme/tokens.toml` changes) and embeds a Windows icon via `winres` (Windows-only build dependency). `toml` is a build-dependency. For macOS distribution use `cargo-bundle`. Release profile uses `lto = true`, `strip = true`.
