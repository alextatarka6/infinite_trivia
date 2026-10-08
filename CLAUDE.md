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

# Unit tests (answer matching, data parsing)
cargo test

# Also validate board generation against the real 55 MB dataset (use --release; slow in debug)
cargo test --release -- --ignored
```

Unit tests cover the pure logic (`game/answer.rs`, `api/jeopardy.rs`, `api/qbreader.rs`). For UI and game-flow changes, also run the app and exercise the relevant mode.

**Dev screen gallery:** `IT_DEV=1 cargo run` boots into a dev gallery (`src/screens/dev.rs`, `Screen::DevMenu`) with buttons that jump straight into any in-game screen/state using mock data — no API or play-through needed. `IT_DEV=1 IT_SCREEN=<key> cargo run` opens one screen directly; keys: `trivia`, `trivia-reveal`, `trivia-summary`, `trivia-loading`, `qb`/`qb-reading`/`qb-buzzed`/`qb-correct`/`qb-incorrect`/`qb-timeout`, `bonus`, `setup`, `board`, `clue`, `clue-result`, `dd`, `dd-clue`, `dd-result`, `final`, `final-result`, `final-teams`/`final-teams-clue`/`final-teams-result` (phone Final), `mix`, `mix-summary`, `lobby`, `standings`. Use this to eyeball UI work instead of scripting clicks.

**Screenshots (debug builds):** `IT_SHOT=out.png` saves a PNG of the window ~1.5s after launch and quits (`IT_SHOT_AT=<secs>` changes the delay). Combine with `IT_DEV=1 IT_SCREEN=<key>` to capture any screen, or run alone for Home.

**Phone controller testing (debug builds):** `IT_PHONE_TEST=7878 cargo run` runs only the phone server, headless, with test hooks: `POST /dev/push[?to=ID]` (raw `ServerMsg` JSON to every seat or one), `POST /dev/drop` (hang up every socket, like a screen lock), `GET /dev/state` (seats + raw client messages). Drive `assets/phone.html` against it with Playwright at phone viewports.

## Architecture

**Stack:** Rust, `egui 0.29` / `eframe 0.29` immediate-mode GUI. `tokio` + `reqwest` for async API calls. `serde_json` for Jeopardy data and future multiplayer serialization.

**Entry point:** `src/main.rs` → `eframe::run_native` → `app::App`. The `App::update()` is called every frame; `theme::apply(ctx)` is called at the top of every frame to enforce the Jeopardy-inspired blue/gold palette.

**Screen routing:** `game/state.rs` defines `enum Screen { Home, JeopardySetup, Jeopardy, Trivia, Quizbowl, DevMenu }`. `app.rs` matches on `self.screen` inside `update()` and delegates to the appropriate subsystem.

**Design system — "Roster" (`src/screens/kit.rs`):** an esports-broadcast look in a Jackbox-style show frame. Flat fills only: no gradients, glows, rounded corners, pills, hover lifts or looping animations. Every shape is a convex polygon or line segment:
- **`Canvas`** maps the 1280×800 design canvas onto the window (uniform scale, centered). Lay screens out in design pixels: `c.p(x, y)`, `c.r(x, y, w, h)`, `c.v(len)`, `c.cond(weight, size)`, `c.barlow(weight, size)`. Full-bleed elements use `c.screen`.
- **Shapes:** `pattern_a` (chamfer top-left + bottom-right: buttons, tiles, plates), `pattern_b` (bottom-right only: roster cards), `slant_pts` (Home mode cards), `tag_pts`, `chamfer_rect`/`slant_rect`, `inner_stroke`. Textures (`Texture::Grid` Jeopardy, `Hatch` Trivia, `Scan` Quizbowl, `Checker` Mix) are line patterns clipped to any convex polygon with `texture(...)`.
- **Components:** `backdrop(ctx, Option<Texture>)` (call **first**: the CentralPanel shares its layer, so anything painted earlier is covered), `button(ui, c, rect, label, Btn, id)` (hover changes fill only; a leading `✓ `/`✕ ` is drawn as line art), `key_hint` (a trailing `↵` is drawn as line art; the bundled fonts lack these glyphs), `segmented`, `tag`, `label`/`text_tracked` (letter-spacing via `extra_letter_spacing`), `photo_plate` (uploaded photo or default crest + monogram), `roster(...)` (the signature bottom row: the active card grows 112→142px and turns gold, scores count up via `count_up`), `stinger` (rotated band, rotated text via `TextShape::angle`), `skew_ticks` (timer), `answer_field`/`answer_field_bare`, `loading(...)`, `round_complete(...)` (winner card + standings + stat tiles; `summary` is the solo wrapper), `quit_button` (top-right, also Esc), `app_icon`, `qr_modules`/`draw_qr`.
- **Motion:** `start_sweep`/`draw_sweep`: App fires the gold stinger sweep whenever `view_identity()` changes (DD/Final bring their own band). Board tiles fill in diagonally; verdict bands flip in. Drive animations off `ui.input(|i| i.time)` / `animate_bool_with_time` and `request_repaint()`.
- Team photos are an extension point: `kit::set_team_photo(ctx, name, texture)`; `game/session.rs` `Player` carries `monogram` + `photo`, and `Crest::for_name` picks a default crest pattern.

**Jeopardy mode — board/overlay separation:** The board (`jeopardy_board: Option<Board>`) is kept alive throughout the game. Clue, Daily Double, and Final Jeopardy screens are stored as `jeopardy_overlay: Option<JeopardyOverlay>` and swapped in/out. This preserves the `used[][]` tile state across clue visits. Score lives on the `Board` struct.

**Redraw policy:** the app does not redraw every frame. `kit::since`, `count_up`, the sweep and `loading` request frames while they animate; async results and phone input (`GameServer::set_repaint_ctx`) wake the UI; `App::timer_running` keeps frames coming only while a trivia countdown or the tossup reader runs. `repaint_on_widget_change` is off because it kept the UI redrawing forever. A new timer-driven state must be added to `timer_running`, or it will freeze until the mouse moves.

**Async pattern:** API calls (`opentdb::fetch_questions`, `qbreader::fetch_tossup`) are spawned onto `self.rt` (a `tokio::Runtime`) and write results into `Arc<Mutex<Option<Result<…>>>>` slots. `App::update()` polls these slots each frame via `try_lock()` and promotes the result into the relevant screen struct.

**Jeopardy data:** `assets/JEOPARDY_QUESTIONS1.json` (~55 MB, 216 k records) is parsed on a background thread at startup (`App::poll_store` promotes it; Jeopardy shows a loading screen until then). It's found via `IT_JEOPARDY_JSON`, then `assets/` beside the executable, a macOS bundle's `Resources/assets`, the repo root, then the working directory. Records are grouped into **columns** keyed by (show, round, category), so a board column is one category as it aired in one episode. `random_board()` picks six full, text-only columns (no "seen here" media clues) with distinct names and sets each clue's `value` from its row (`jeopardy_value`), so the board, phones and scoring agree. Clue text is shown as written (HTML stripped, entities decoded); never "auto-correct" it. JSON fields are `category`, `air_date`, `question`, `value` (`"$200"`, `"$1,000"`, or `null`/`"None"`), `answer`, `round`, `show_number`.

**Scoring (`game/scoring.rs`):**
- Answer judging (`game/answer.rs`, shared by Jeopardy and Quizbowl): word-based, never substring. Normalizes case, accents, punctuation, "what is", leading articles and number words; accepts typos by length, a superset of the expected words, or a 4+ letter surname. Empty input is always wrong.
- Jeopardy: clue face value, negated on wrong answer; Daily Double wager is capped at max(score, round's top value). Final Jeopardy: solo types the wager/answer on the host; phone games collect a secret wager and an answer from every team above $0, then show results and the final standings screen (PLAY AGAIN keeps the teams).
- Trivia: +10 per correct answer, streak multiplier kicks in every 3 in a row (max 4×).
- Quizbowl: correct = +10; a wrong answer before the tossup finishes = −5 (`quizbowl_score`). The tossup screen shows exactly these values (the hi-fi's 20/15/10 power ladder is not implemented). A correct tossup is followed by a **bonus round** (`screens/quizbowl/bonus.rs`, real qbreader `/api/random-bonus`): the player answers 3 parts solo, +10 each (the "other players steal on a miss" multiplayer rule is deferred to the future Jackbox layer). `app.rs` orchestrates it via `quizbowl_bonus: Option<BonusScreen>` + `pending_bonus`, swapping the bonus in for the tossup view until it returns `BonusAction::Done`, then loading the next tossup.

**Design tokens (`theme/tokens.toml` → `build.rs` → `src/theme.rs`):** The palette, shape, textures, spacing and type scale are defined once in `theme/tokens.toml` (the single source of truth). `build.rs` generates `$OUT_DIR/design_tokens.rs` with compile-time `pub const`s — hex → `Color32::from_rgb`, base+alpha → premultiplied `Color32`, scalars → `CANVAS_*` / `CHAMFER_*` / `SLANT_*` / `TEXTURE_*` / `SPACE_*` / `TYPE_*`. `theme.rs` `include!`s them and builds the flat, square-cornered egui `Style`. **To change any color/size, edit `tokens.toml` and rebuild — never hand-edit the generated constants or hardcode hex in screens.** Colors: `BG` (screen), `PANEL` (cards, segments), `PANEL_HI` (live tiles, plates), `GOLD`/`GOLD_HI`/`ON_GOLD`, `INK`/`MUTED` (text), `GOOD`/`ON_GOOD`, `BAD`/`ON_BAD`, `LINE` (hairlines). Fonts: `theme::cond(weight, size)` = Barlow Condensed 500–800 (headlines, numbers, labels, buttons; caps) and `theme::barlow(weight, size)` = Barlow 400–600 (body, tossup text), compiled in from `assets/fonts/*.ttf` (SIL OFL, `assets/fonts/OFL.txt`) via `include_bytes!` in `App::new`. `assets/phone.html` mirrors the colors as CSS variables — keep them in sync. The app icon is `assets/icon.png` (window icon in `main.rs`, logo on Home).

**Design reference (`design/`, gitignored):** the current target is `design/roster/Roster Hi-fi.dc.html` (8 screens at 1280×800: Home, Board, Clue, Daily Double, Trivia, Quizbowl, Summary, Loading); read its inline styles for exact sizes. `design/app/` holds the older React prototype (superseded). Neither is committed — only `theme/tokens.toml` is.

**Porting the hi-fi HTML → egui:** screens are painter-first on a `Canvas`: copy absolute positions/sizes from the HTML's inline styles into `c.r(...)` calls, draw shapes with the kit polygon helpers (`clip-path: polygon(...)` → `chamfer_pts`/`slant_pts`/`tag_pts`), and hit-test with `ui.interact(rect, id, Sense::click())` (`kit::button` does this). `letter-spacing: .2em` → `text_tracked(..., 0.2)`. `transform: rotate()` on text → `text_rotated`. Keep left-weighted, asymmetric layouts; don't center content columns.

**Phone multiplayer (`src/server/`, `assets/phone.html`):** picking TEAMS on Home starts an axum server on port 7777 (`App::ensure_server`) serving the phone controller page, its fonts (`/fonts/*`) and a WebSocket (`/ws`). Phones keep a token in `localStorage` and send `hello {token}` on every (re)connect; the server maps tokens to **seats**, so a screen lock or reload returns the player to the same seat, and the server replays that seat's last game-state message. Disconnects mark a seat `connected: false` rather than removing it. `GameServer::broadcast` sends one message to all seats; `send_each(|id| msg)` builds a per-player message (`is_you`, `is_your_turn`). `App::tick_multiplayer` dispatches to the per-mode sync structs in `src/multiplayer.rs` (`TriviaSync`, `QuizbowlSync`, `ClueSync` for Mix, `JeopardySync` state with the tick in `app.rs`). Every tick drains the input queue even while loading, so stale input can't hit the next question; `Multiplayer::new_game` resets sync state and seat scores when a game starts. For Jeopardy the board is the score ledger and is mirrored onto seats each tick. Modes: Trivia collects one answer per phone (scored with that phone's streak) and reveals when everyone has answered or the host's question resolves; Quizbowl sends `is_you` to the buzzer only; Jeopardy maps board seats to phone ids (`JeopardySync::player_ids`) so only the active seat may pick, wager or answer, except in Final, where every seat plays; Mix sends its Jeopardy clues to every phone and the first response is judged. If port 7777 is busy the server tries the next nine; failures show in the lobby. The phone page patches streaming text in place (it never rebuilds the BUZZ button or a focused input), and BUZZ fires on `pointerdown`.

**Scaffold (intentionally unused):** `game/session.rs` (`Session`) and `game/state.rs` (`GameState`) are extension points; dead-code warnings for them are expected and should not be removed.

**Platform notes:** `build.rs` does two jobs: generates the design-token constants (all platforms; reruns when `theme/tokens.toml` changes) and embeds a Windows icon via `winres` (Windows-only build dependency). `toml` is a build-dependency. For macOS distribution use `cargo-bundle`. Release profile uses `lto = true`, `strip = true`.
