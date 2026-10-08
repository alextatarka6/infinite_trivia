use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use egui::Context;
use crate::api::{jeopardy::JeopardyStore, opentdb, qbreader};
use crate::game::state::Screen;
use crate::screens::{
    kit,
    dev::DevMenu,
    home::{Couch, HomeAction, HomeScreen},
    lobby::{LobbyAction, LobbyMode, LobbyScreen},
    jeopardy::{
        board::{Board, BoardAction},
        clue::ClueScreen,
        daily_double::DailyDoubleScreen,
        final_jeopardy::FinalJeopardy,
        setup::{JeopardyConfig, JeopardyMode, JeopardySetupScreen, SetupAction},
    },
    mix::{MixPhase, MixScreen, MixSignal},
    trivia::question::TriviaScreen,
    quizbowl::{bonus::BonusScreen, tossup::TossupScreen},
};
use crate::multiplayer::{self, Multiplayer};
use crate::server::{protocol::{ClientMsg, ServerMsg}, GameServer};
use crate::theme;

const JSON_NAME: &str = "JEOPARDY_QUESTIONS1.json";

/// Desktop typefaces, compiled in so the app looks right from any directory.
const FONTS: &[(&str, &[u8])] = &[
    ("cond-500", include_bytes!("../assets/fonts/BarlowCondensed-Medium.ttf")),
    ("cond-600", include_bytes!("../assets/fonts/BarlowCondensed-SemiBold.ttf")),
    ("cond-700", include_bytes!("../assets/fonts/BarlowCondensed-Bold.ttf")),
    ("cond-800", include_bytes!("../assets/fonts/BarlowCondensed-ExtraBold.ttf")),
    ("barlow-400", include_bytes!("../assets/fonts/Barlow-Regular.ttf")),
    ("barlow-500", include_bytes!("../assets/fonts/Barlow-Medium.ttf")),
    ("barlow-600", include_bytes!("../assets/fonts/Barlow-SemiBold.ttf")),
];

/// Find the Jeopardy data: `IT_JEOPARDY_JSON`, then `assets/` beside the
/// executable, in a macOS bundle's Resources, at the repo root (for
/// `target/<profile>/`), and finally relative to the working directory.
fn jeopardy_json_path() -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Some(p) = std::env::var_os("IT_JEOPARDY_JSON") {
        return PathBuf::from(p);
    }
    let mut candidates = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.to_path_buf())) {
        candidates.push(dir.join("assets").join(JSON_NAME));
        candidates.push(dir.join("../Resources/assets").join(JSON_NAME));
        candidates.push(dir.join("../../assets").join(JSON_NAME));
    }
    let cwd = PathBuf::from("assets").join(JSON_NAME);
    candidates.into_iter().find(|p| p.exists()).unwrap_or(cwd)
}

type StoreSlot = Arc<Mutex<Option<Result<JeopardyStore, String>>>>;

enum JeopardyOverlay {
    Clue(ClueScreen),
    DailyDouble(DailyDoubleScreen),
    Final(FinalJeopardy),
}

pub struct App {
    screen: Screen,
    home_screen: HomeScreen,
    dev_menu: DevMenu,
    jeopardy_store: Option<JeopardyStore>,
    jeopardy_store_error: Option<String>,
    /// Filled by the background thread that parses the Jeopardy data.
    pending_store: StoreSlot,
    jeopardy_setup: Option<JeopardySetupScreen>,
    jeopardy_board: Option<Board>,
    jeopardy_overlay: Option<JeopardyOverlay>,
    /// Config of the running game, reused by PLAY AGAIN on the standings screen.
    jeopardy_config: Option<JeopardyConfig>,
    /// Set (to the time it appeared) once Final is done: shows final standings.
    jeopardy_over: Option<f64>,
    daily_doubles: Vec<(usize, usize)>,
    trivia: Option<TriviaScreen>,
    trivia_level: u8,
    quizbowl: Option<TossupScreen>,
    quizbowl_bonus: Option<BonusScreen>,
    quizbowl_level: u8,
    recent_qb_categories: VecDeque<String>,
    mix: Option<MixScreen>,
    mix_level: u8,
    // Multiplayer
    server: Option<Arc<GameServer>>,
    lobby: Option<LobbyScreen>,
    /// Why the phone server isn't running, if starting it failed.
    server_error: Option<String>,
    mp: Multiplayer,
    /// Identifies the visible view; a change triggers the stinger sweep.
    view_key: &'static str,
    rt: tokio::runtime::Runtime,
    pending_trivia: Arc<Mutex<Option<Result<Vec<crate::api::opentdb::TriviaQuestion>, String>>>>,
    pending_qb: Arc<Mutex<Option<Result<crate::api::qbreader::Tossup, String>>>>,
    pending_bonus: Arc<Mutex<Option<Result<crate::api::qbreader::Bonus, String>>>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load custom typefaces
        {
            let mut fonts = egui::FontDefinitions::default();

            for &(key, bytes) in FONTS {
                fonts.font_data.insert(key.to_owned(), egui::FontData::from_static(bytes));
                // Fall back to egui's bundled fonts for glyphs Barlow lacks (∞, ↵, ✓).
                let mut chain = vec![key.to_owned()];
                chain.extend(fonts.families[&egui::FontFamily::Proportional].iter().cloned());
                fonts.families.insert(egui::FontFamily::Name(key.into()), chain);
            }

            // Barlow is the default UI face; condensed bold for monospace-ish numerals.
            fonts.families.entry(egui::FontFamily::Proportional).or_default()
                .insert(0, "barlow-500".to_owned());
            fonts.families.entry(egui::FontFamily::Monospace).or_default()
                .insert(0, "cond-700".to_owned());

            cc.egui_ctx.set_fonts(fonts);
        }
        // Screens are painted at fixed canvas positions, so egui's "a widget
        // moved, draw another frame" check only keeps the UI redrawing forever.
        cc.egui_ctx.options_mut(|o| o.repaint_on_widget_change = false);

        // Parsing ~55 MB of JSON takes a moment: do it off the UI thread.
        let pending_store: StoreSlot = Arc::new(Mutex::new(None));
        {
            let slot = Arc::clone(&pending_store);
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                let result = JeopardyStore::load(&jeopardy_json_path());
                *slot.lock().unwrap() = Some(result);
                ctx.request_repaint();
            });
        }

        Self {
            screen: if std::env::var("IT_DEV").is_ok() {
                Screen::DevMenu
            } else {
                Screen::Home
            },
            home_screen: HomeScreen::default(),
            dev_menu: DevMenu::default(),
            jeopardy_store: None,
            jeopardy_store_error: None,
            pending_store,
            jeopardy_setup: None,
            jeopardy_board: None,
            jeopardy_overlay: None,
            jeopardy_config: None,
            jeopardy_over: None,
            daily_doubles: vec![],
            trivia: None,
            trivia_level: 2,
            quizbowl: None,
            quizbowl_bonus: None,
            quizbowl_level: 2,
            recent_qb_categories: VecDeque::new(),
            mix: None,
            mix_level: 2,
            server: None,
            lobby: None,
            server_error: None,
            mp: Multiplayer::default(),
            view_key: "",
            rt: tokio::runtime::Runtime::new().expect("tokio runtime"),
            pending_trivia: Arc::new(Mutex::new(None)),
            pending_qb: Arc::new(Mutex::new(None)),
            pending_bonus: Arc::new(Mutex::new(None)),
        }
    }

    fn start_jeopardy(&mut self, config: JeopardyConfig) {
        self.jeopardy_config = Some(config.clone());
        self.jeopardy_over = None;
        let store = match &self.jeopardy_store {
            Some(s) => s,
            None => return,
        };

        let players: Vec<(String, i32)> = match config.mode {
            JeopardyMode::Solo => vec![("You".to_string(), 0)],
            JeopardyMode::Teams => (0..config.team_count as usize)
                .map(|i| {
                    let name = config.team_names
                        .get(i)
                        .cloned()
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or_else(|| format!("Team {}", i + 1));
                    (name, 0)
                })
                .collect(),
        };

        match store.random_board("Jeopardy!") {
            Some(cats) => {
                let n_cols = cats.len();
                let n_rows = cats.first().map(|(_, v)| v.len()).unwrap_or(5);
                self.daily_doubles = Self::pick_daily_doubles(n_cols, n_rows, 1);
                self.jeopardy_board = Some(Board::new(cats, false, players));
                self.jeopardy_overlay = None;
            }
            None => eprintln!("No Jeopardy! round data found in JSON"),
        }
    }

    /// `count` Daily Doubles in distinct columns, never on the top row.
    fn pick_daily_doubles(cols: usize, rows: usize, count: usize) -> Vec<(usize, usize)> {
        let mut columns: Vec<usize> = (0..cols).collect();
        fastrand::shuffle(&mut columns);
        columns
            .into_iter()
            .take(count)
            .map(|col| (col, 1 + fastrand::usize(..rows.saturating_sub(1).max(1))))
            .collect()
    }

    fn start_trivia(&mut self, ctx: Context, level: u8) {
        self.trivia_level = level;
        self.trivia = Some(TriviaScreen::loading());
        let pending = Arc::clone(&self.pending_trivia);
        let diff = match level {
            1 => "easy",
            3 => "hard",
            4 => "",
            _ => "medium",
        }
        .to_string();
        self.rt.spawn(async move {
            let result = opentdb::fetch_questions(10, &diff).await;
            *pending.lock().unwrap() = Some(result);
            ctx.request_repaint();
        });
    }

    fn start_quizbowl(&mut self, ctx: Context) {
        let (score, done) = self
            .quizbowl
            .as_ref()
            .map(|q| (q.score, q.questions_done))
            .unwrap_or((0, 0));
        self.quizbowl = Some(TossupScreen::loading(score, done));
        let pending = Arc::clone(&self.pending_qb);
        let level = self.quizbowl_level;
        let exclude: Vec<String> = self.recent_qb_categories.iter().cloned().collect();
        self.rt.spawn(async move {
            let result = qbreader::fetch_tossup(level, exclude).await;
            *pending.lock().unwrap() = Some(result);
            ctx.request_repaint();
        });
    }

    fn start_bonus(&mut self, ctx: Context, score: i32, done: u32) {
        self.quizbowl_bonus = Some(BonusScreen::loading(score, done));
        let pending = Arc::clone(&self.pending_bonus);
        let level = self.quizbowl_level;
        self.rt.spawn(async move {
            let result = qbreader::fetch_bonus(level).await;
            *pending.lock().unwrap() = Some(result);
            ctx.request_repaint();
        });
    }

    fn start_mix(&mut self, ctx: Context, level: u8) {
        self.mix_level = level;
        let jeopardy_clues = self
            .jeopardy_store
            .as_ref()
            .map(|s| s.random_clues("Jeopardy!", crate::screens::mix::JEOPARDY_COUNT))
            .unwrap_or_default();
        self.mix = Some(MixScreen::new(jeopardy_clues));
        let pending = Arc::clone(&self.pending_trivia);
        let diff = match level {
            1 => "easy",
            3 => "hard",
            4 => "",
            _ => "medium",
        }
        .to_string();
        self.rt.spawn(async move {
            let result = opentdb::fetch_questions(
                crate::screens::mix::TRIVIA_COUNT as u8,
                &diff,
            )
            .await;
            *pending.lock().unwrap() = Some(result);
            ctx.request_repaint();
        });
    }

    fn fetch_mix_qb(&mut self, ctx: Context, exclude: Option<String>) {
        let pending = Arc::clone(&self.pending_qb);
        let level = self.mix_level;
        let exclude_list: Vec<String> = exclude.into_iter().collect();
        self.rt.spawn(async move {
            let result = qbreader::fetch_tossup(level, exclude_list).await;
            *pending.lock().unwrap() = Some(result);
            ctx.request_repaint();
        });
    }

    /// Record the just-played tossup category (for variety) and load the next one.
    fn next_tossup(&mut self, ctx: Context) {
        if let Some(ref qb) = self.quizbowl {
            let cat = qb.tossup.category.clone();
            if !cat.is_empty() {
                self.recent_qb_categories.push_back(cat);
                if self.recent_qb_categories.len() > 4 {
                    self.recent_qb_categories.pop_front();
                }
            }
        }
        self.start_quizbowl(ctx);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        theme::apply(ctx);
        self.poll_store();

        // Poll async trivia result (standalone mode only)
        if self.screen == Screen::Trivia {
            if let Ok(mut guard) = self.pending_trivia.try_lock() {
                if let Some(result) = guard.take() {
                    self.trivia = Some(match result {
                        Ok(qs) => TriviaScreen::new(qs),
                        Err(e) => TriviaScreen::with_error(e),
                    });
                }
            }
        }

        // Poll async quizbowl result (standalone mode only)
        if self.screen == Screen::Quizbowl {
            if let Ok(mut guard) = self.pending_qb.try_lock() {
                if let Some(result) = guard.take() {
                    let (score, done) = self
                        .quizbowl
                        .as_ref()
                        .map(|q| (q.score, q.questions_done))
                        .unwrap_or((0, 0));
                    self.quizbowl = Some(match result {
                        Ok(t) => TossupScreen::new(t, score, done),
                        Err(e) => {
                            let mut s = TossupScreen::loading(score, done);
                            s.loading = false;
                            s.error = Some(e);
                            s
                        }
                    });
                }
            }
        }

        // Poll async bonus result (standalone quizbowl mode only)
        if self.screen == Screen::Quizbowl {
            if let Ok(mut guard) = self.pending_bonus.try_lock() {
                if let Some(result) = guard.take() {
                    if let Some((score, done)) =
                        self.quizbowl_bonus.as_ref().map(|b| (b.score, b.questions_done))
                    {
                        self.quizbowl_bonus = Some(match result {
                            Ok(b) => BonusScreen::new(b, score, done),
                            Err(e) => {
                                let mut s = BonusScreen::loading(score, done);
                                s.loading = false;
                                s.error = Some(e);
                                s
                            }
                        });
                    }
                }
            }
        }

        // Poll async results for mix mode
        if self.screen == Screen::Mix {
            if matches!(self.mix.as_ref().map(|m| &m.phase), Some(MixPhase::Loading)) {
                if let Ok(mut guard) = self.pending_trivia.try_lock() {
                    if let Some(result) = guard.take() {
                        if let Some(mix) = self.mix.as_mut() {
                            mix.on_trivia_loaded(result);
                        }
                    }
                }
            }
            if matches!(self.mix.as_ref().map(|m| &m.phase), Some(MixPhase::QuizbowlLoading)) {
                if let Ok(mut guard) = self.pending_qb.try_lock() {
                    if let Some(result) = guard.take() {
                        if let Some(mix) = self.mix.as_mut() {
                            mix.on_qb_loaded(result);
                        }
                    }
                }
            }
        }

        let dt = ctx.input(|i| i.stable_dt);

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(theme::BG))
            .show(ctx, |ui| match self.screen {
                Screen::Home => {
                    let url = self.server.as_ref().map(|s| s.url());
                    let players = self.server.as_ref().map(|s| s.players()).unwrap_or_default();
                    let couch = Couch { url: url.as_deref(), players: &players };
                    match self.home_screen.show(ui, couch) {
                        Some(HomeAction::OpenTeams) => self.ensure_server(ctx),
                        Some(HomeAction::Start { mode, level, solo }) => {
                            self.start_from_home(mode, level, solo, ctx)
                        }
                        None => {}
                    }
                }

                Screen::JeopardySetup => {
                    if let Some(ref mut setup) = self.jeopardy_setup {
                        match setup.show(ui) {
                            Some(SetupAction::Start(config)) => {
                                self.start_jeopardy(config);
                                self.screen = Screen::Jeopardy;
                                self.jeopardy_setup = None;
                            }
                            Some(SetupAction::Back) => {
                                self.screen = Screen::Home;
                                self.jeopardy_setup = None;
                            }
                            None => {}
                        }
                    }
                }

                Screen::Jeopardy => {
                    self.update_jeopardy(ui, ctx);
                }

                Screen::Trivia => {
                    let phones: Vec<(String, i32)> = self.server.as_ref()
                        .map(|s| s.players().into_iter().map(|p| (p.name, p.score)).collect())
                        .unwrap_or_default();
                    if let Some(ref mut trivia) = self.trivia {
                        trivia.phones = phones;
                        use crate::screens::trivia::question::TriviaAction;
                        if let Some(action) = trivia.show(ui, dt) {
                            match action {
                                TriviaAction::Quit => self.screen = Screen::Home,
                                TriviaAction::Restart => {
                                    self.mp.new_game(self.server.as_deref());
                                    self.start_trivia(ctx.clone(), self.trivia_level);
                                }
                                TriviaAction::Next { .. } => {}
                            }
                        }
                    }
                }

                Screen::Quizbowl => {
                    self.update_quizbowl(ui, ctx);
                }

                Screen::Mix => {
                    self.update_mix(ui, ctx);
                }

                Screen::Lobby => {
                    self.update_lobby(ui, ctx);
                }

                Screen::DevMenu => {
                    if self.dev_menu.show(ui, dt) {
                        self.screen = Screen::Home;
                    }
                }
            });

        // Stinger sweep on every screen change (screens with their own band skip it).
        let (key, label) = self.view_identity();
        if key != self.view_key {
            let back_to_board = key == "board" && matches!(self.view_key, "clue" | "dd" | "final");
            if !matches!(key, "dd" | "final" | "dev") {
                kit::start_sweep(ctx, if back_to_board { "" } else { label });
            }
            self.view_key = key;
        }
        kit::draw_sweep(ctx);

        // Multiplayer: broadcast game state and handle phone inputs each frame
        self.tick_multiplayer(ctx);

        #[cfg(debug_assertions)]
        dev_screenshot(ctx);

        // Animations request their own frames, async results and phone input
        // wake the UI, so only running timers need continuous redraws.
        if self.timer_running() {
            ctx.request_repaint();
        }
    }
}

impl App {
    /// Promote the background-loaded Jeopardy data, starting any game or Mix
    /// round that was waiting for it.
    fn poll_store(&mut self) {
        let Some(result) = self.pending_store.try_lock().ok().and_then(|mut g| g.take()) else { return };
        match result {
            Ok(store) => self.jeopardy_store = Some(store),
            Err(e) => self.jeopardy_store_error = Some(e),
        }
        if self.screen == Screen::Jeopardy && self.jeopardy_board.is_none() {
            if let Some(config) = self.jeopardy_config.clone() {
                self.start_jeopardy(config);
            }
        }
        if let (Some(store), Some(mix)) = (&self.jeopardy_store, self.mix.as_mut()) {
            mix.fill_jeopardy_clues(|| store.random_clues("Jeopardy!", crate::screens::mix::JEOPARDY_COUNT));
        }
    }

    /// True while a countdown or the tossup reader is running.
    fn timer_running(&self) -> bool {
        #[cfg(debug_assertions)]
        if std::env::var_os("IT_SHOT").is_some() {
            return true;
        }
        let trivia_live = |t: &TriviaScreen| {
            !t.loading && t.error.is_none() && t.current_idx < t.questions.len() && t.selected.is_none()
        };
        let qb_live = |q: &TossupScreen| {
            use crate::screens::quizbowl::tossup::TossupState;
            !q.loading && matches!(q.state, TossupState::Reading | TossupState::FinishedReading { .. })
        };
        match self.screen {
            Screen::Trivia => self.trivia.as_ref().is_some_and(trivia_live),
            Screen::Quizbowl => self.quizbowl_bonus.is_none() && self.quizbowl.as_ref().is_some_and(qb_live),
            Screen::Mix => self.mix.as_ref().is_some_and(|m| match m.phase {
                MixPhase::Trivia => m.trivia.as_ref().is_some_and(trivia_live),
                MixPhase::Quizbowl => m.quizbowl.as_ref().is_some_and(qb_live),
                _ => false,
            }),
            Screen::DevMenu => true,
            _ => false,
        }
    }

    fn update_jeopardy(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        if let Some(err) = self.jeopardy_store_error.clone() {
            kit::backdrop(ui.ctx(), None);
            if crate::screens::trivia::question::error_screen(ui, "COULD NOT LOAD JEOPARDY DATA", &err) {
                self.screen = Screen::Home;
            }
            return;
        }

        if self.jeopardy_store.is_none() {
            kit::loading(ui, "LOADING CLUES", "JEOPARDY! · 216,000 CLUES", None);
            return;
        }

        if let Some(start) = self.jeopardy_over {
            self.show_jeopardy_standings(ui, start);
            return;
        }

        // Overlays draw the roster row, so hand them the board's teams.
        let teams = self.jeopardy_board.as_ref().map(|b| (b.players.clone(), b.active)).unwrap_or_default();
        let overlay_done = match self.jeopardy_overlay.take() {
            Some(JeopardyOverlay::Clue(mut s)) => {
                (s.teams, s.active) = teams.clone();
                use crate::screens::jeopardy::clue::ClueAction;
                match s.show(ui) {
                    Some(ClueAction::Done { correct }) => {
                        if let Some(board) = &mut self.jeopardy_board {
                            let delta = if correct { s.clue.value } else { -s.clue.value };
                            board.players[board.active].1 += delta;
                            board.advance_turn();
                        }
                        true
                    }
                    None => {
                        self.jeopardy_overlay = Some(JeopardyOverlay::Clue(s));
                        false
                    }
                }
            }

            Some(JeopardyOverlay::DailyDouble(mut s)) => {
                (s.teams, s.active) = teams.clone();
                use crate::screens::jeopardy::daily_double::DailyDoubleAction;
                match s.show(ui) {
                    Some(DailyDoubleAction::Done { winnings }) => {
                        if let Some(board) = &mut self.jeopardy_board {
                            board.players[board.active].1 += winnings;
                            board.advance_turn();
                        }
                        true
                    }
                    None => {
                        self.jeopardy_overlay = Some(JeopardyOverlay::DailyDouble(s));
                        false
                    }
                }
            }

            Some(JeopardyOverlay::Final(mut s)) => {
                use crate::screens::jeopardy::final_jeopardy::FinalAction;
                match s.show(ui) {
                    Some(FinalAction::Done { deltas }) => {
                        if let Some(board) = &mut self.jeopardy_board {
                            for (player, d) in board.players.iter_mut().zip(deltas) {
                                player.1 += d;
                            }
                        }
                        self.jeopardy_over = Some(ui.input(|i| i.time));
                        return;
                    }
                    None => {
                        self.jeopardy_overlay = Some(JeopardyOverlay::Final(s));
                        false
                    }
                }
            }

            None => false,
        };

        if overlay_done || self.jeopardy_overlay.is_none() {
            if let Some(b) = self.jeopardy_board.as_mut() {
                b.dd_left = self.daily_doubles.iter().filter(|&&(c, r)| !b.used[c][r]).count();
            }
            let action = self.jeopardy_board.as_mut().and_then(|b| b.show(ui));
            if let Some(BoardAction::SelectClue { col, row }) = action {
                self.open_clue(col, row);
            } else if let Some(board) = self.jeopardy_board.as_mut() {
                match action {
                    Some(BoardAction::SelectClue { .. }) => {}

                    Some(BoardAction::GoToFinal) => {
                        if board.is_double_jeopardy {
                            if let Some(store) = &self.jeopardy_store {
                                if let Some(clue) = store.random_final() {
                                    let phones = !self.mp.jeopardy.player_ids.is_empty();
                                    self.jeopardy_overlay = Some(JeopardyOverlay::Final(
                                        FinalJeopardy::new(clue, board.players.clone(), phones),
                                    ));
                                }
                            }
                        } else {
                            let prev_players = board.players.clone();
                            let prev_active = board.active;
                            if let Some(store) = &self.jeopardy_store {
                                if let Some(cats) = store.random_board("Double Jeopardy!") {
                                    let n_cols = cats.len();
                                    let n_rows =
                                        cats.first().map(|(_, v)| v.len()).unwrap_or(5);
                                    self.daily_doubles =
                                        Self::pick_daily_doubles(n_cols, n_rows, 2);
                                    let mut new_board = Board::new(cats, true, prev_players);
                                    new_board.active = prev_active;
                                    *board = new_board;
                                }
                            }
                        }
                    }

                    Some(BoardAction::Quit) => {
                        self.screen = Screen::Home;
                        self.jeopardy_board = None;
                    }

                    None => {}
                }
            }
        }

        let _ = ctx;
    }

    /// Final standings after Final Jeopardy: PLAY AGAIN keeps the same teams.
    fn show_jeopardy_standings(&mut self, ui: &mut egui::Ui, start: f64) {
        let mut players = self.jeopardy_board.as_ref().map(|b| b.players.clone()).unwrap_or_default();
        players.sort_by(|a, b| b.1.cmp(&a.1));
        let standings: Vec<kit::Standing> = players
            .iter()
            .map(|(n, s)| kit::Standing { name: n.clone(), score: kit::money(*s), negative: *s < 0 })
            .collect();
        let top = players.first().map(|p| p.1).unwrap_or(0);
        let stats = [
            kit::Stat { value: kit::money(top), label: "winning total" },
            kit::Stat { value: players.len().to_string(), label: if players.len() == 1 { "player" } else { "teams" } },
        ];
        match kit::round_complete(ui, "Jeopardy!", &standings, &stats, start) {
            kit::SummaryAction::Restart => {
                if let Some(config) = self.jeopardy_config.clone() {
                    self.mp.new_game(self.server.as_deref());
                    self.start_jeopardy(config);
                }
            }
            kit::SummaryAction::Quit => {
                self.screen = Screen::Home;
                self.jeopardy_board = None;
                self.jeopardy_over = None;
            }
            kit::SummaryAction::None => {}
        }
    }

    /// (key, sweep label) for the visible view. DD / Final bring their own band,
    /// so `update` skips the sweep for them.
    fn view_identity(&self) -> (&'static str, &'static str) {
        match self.screen {
            Screen::Home => ("home", ""),
            Screen::JeopardySetup => ("setup", "JEOPARDY!"),
            Screen::Jeopardy if self.jeopardy_over.is_some() => ("standings", ""),
            Screen::Jeopardy => match self.jeopardy_overlay {
                Some(JeopardyOverlay::Clue(_)) => ("clue", ""),
                Some(JeopardyOverlay::DailyDouble(_)) => ("dd", ""),
                Some(JeopardyOverlay::Final(_)) => ("final", ""),
                None => ("board", "JEOPARDY!"),
            },
            Screen::Trivia => ("trivia", "TRIVIA"),
            Screen::Quizbowl => ("quizbowl", "QUIZBOWL"),
            Screen::Mix => ("mix", "MIX"),
            Screen::Lobby => ("lobby", "LOBBY"),
            Screen::DevMenu => ("dev", ""),
        }
    }

    /// Mark a board tile used and open its clue (or Daily Double) overlay.
    fn open_clue(&mut self, col: usize, row: usize) {
        let Some(board) = self.jeopardy_board.as_mut() else { return };
        if board.used.get(col).and_then(|c| c.get(row)).copied().unwrap_or(true) {
            return;
        }
        let Some(clue) = board.categories.get(col).and_then(|(_, c)| c.get(row)).cloned() else { return };
        board.used[col][row] = true;
        let active_score = board.active_score();
        self.jeopardy_overlay = Some(if self.daily_doubles.contains(&(col, row)) {
            JeopardyOverlay::DailyDouble(DailyDoubleScreen::new(clue, active_score))
        } else {
            JeopardyOverlay::Clue(ClueScreen::new(clue))
        });
    }

    fn update_quizbowl(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        use crate::screens::quizbowl::bonus::BonusAction;
        use crate::screens::quizbowl::tossup::TossupAction;
        let dt = ctx.input(|i| i.stable_dt);

        // A bonus round, if active, replaces the tossup view.
        if self.quizbowl_bonus.is_some() {
            let action = self.quizbowl_bonus.as_mut().and_then(|b| b.show(ui));
            match action {
                Some(BonusAction::Done { score }) => {
                    // Carry the post-bonus score into the tossup loop, then continue.
                    if let Some(qb) = self.quizbowl.as_mut() {
                        qb.score = score;
                    }
                    self.quizbowl_bonus = None;
                    self.next_tossup(ctx.clone());
                }
                Some(BonusAction::Quit) => {
                    self.screen = Screen::Home;
                    self.quizbowl = None;
                    self.quizbowl_bonus = None;
                }
                None => {}
            }
            return;
        }

        let phones: Vec<(String, i32)> = self.server.as_ref()
            .map(|s| s.players().into_iter().map(|p| (p.name, p.score)).collect())
            .unwrap_or_default();
        if let Some(qb) = self.quizbowl.as_mut() {
            qb.phones = phones;
        }
        let action = self.quizbowl.as_mut().and_then(|qb| qb.show(ui, dt));
        match action {
            Some(TossupAction::Next) => self.next_tossup(ctx.clone()),
            Some(TossupAction::Bonus) => {
                let (score, done) = self
                    .quizbowl
                    .as_ref()
                    .map(|q| (q.score, q.questions_done))
                    .unwrap_or((0, 0));
                self.start_bonus(ctx.clone(), score, done);
            }
            Some(TossupAction::Quit) => {
                self.screen = Screen::Home;
                self.quizbowl = None;
            }
            None => {}
        }
    }

    fn update_lobby(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        let players = self.server.as_ref().map(|s| s.players()).unwrap_or_default();
        let action = self.lobby.as_mut().and_then(|lobby| lobby.show(ui, &players));
        if let Some(action) = action {
            self.run_lobby_action(action, ctx);
        }
    }

    /// Start the phone server (and its lobby screen) if it isn't running yet.
    /// On failure the lobby shows why, and the next TEAMS click retries.
    fn ensure_server(&mut self, ctx: &Context) {
        if self.server.is_some() {
            return;
        }
        match GameServer::start(7777, &self.rt) {
            Ok(srv) => {
                srv.set_repaint_ctx(ctx);
                let mut lobby = LobbyScreen::new(&srv.url());
                if !srv.lan_reachable() {
                    lobby.warning = Some("No Wi-Fi or LAN address found: phones can't reach this computer.".into());
                }
                self.lobby = Some(lobby);
                self.server = Some(srv);
                self.server_error = None;
            }
            Err(e) => {
                let mut lobby = LobbyScreen::new("");
                lobby.warning = Some(e.clone());
                self.lobby = Some(lobby);
                self.server_error = Some(e);
            }
        }
    }

    /// Home's START. Solo games begin immediately; team games use the phones on
    /// the couch, or open the lobby to wait for some if nobody has joined.
    fn start_from_home(&mut self, mode: LobbyMode, level: u8, solo: bool, ctx: &Context) {
        if !solo {
            self.ensure_server(ctx);
            let nobody = self.server.as_ref().is_none_or(|s| s.players().is_empty());
            let action = match mode {
                LobbyMode::Trivia => LobbyAction::StartTrivia(level),
                LobbyMode::Quizbowl => LobbyAction::StartQuizbowl(level),
                LobbyMode::Jeopardy => LobbyAction::StartJeopardy,
                LobbyMode::Mix => LobbyAction::StartMix(level),
            };
            if nobody {
                if let Some(lobby) = self.lobby.as_mut() {
                    lobby.mode = mode;
                    lobby.level = level;
                }
                self.screen = Screen::Lobby;
            } else {
                self.run_lobby_action(action, ctx);
            }
            return;
        }
        self.mp.new_game(self.server.as_deref());
        self.mp.jeopardy.player_ids.clear();
        match mode {
            LobbyMode::Jeopardy => {
                self.start_jeopardy(JeopardyConfig { mode: JeopardyMode::Solo, team_count: 1, team_names: vec![] });
                self.screen = Screen::Jeopardy;
            }
            LobbyMode::Trivia => {
                self.start_trivia(ctx.clone(), level);
                self.screen = Screen::Trivia;
            }
            LobbyMode::Quizbowl => {
                self.quizbowl_level = level;
                self.quizbowl = None;
                self.quizbowl_bonus = None;
                self.recent_qb_categories.clear();
                self.start_quizbowl(ctx.clone());
                self.screen = Screen::Quizbowl;
            }
            LobbyMode::Mix => {
                self.start_mix(ctx.clone(), level);
                self.screen = Screen::Mix;
            }
        }
    }

    fn run_lobby_action(&mut self, action: LobbyAction, ctx: &Context) {
        if !matches!(action, LobbyAction::Back) {
            self.mp.new_game(self.server.as_deref());
        }
        match action {
            LobbyAction::Back => {
                self.screen = Screen::Home;
            }
            LobbyAction::StartTrivia(level) => {
                self.start_trivia(ctx.clone(), level);
                self.screen = Screen::Trivia;
            }
            LobbyAction::StartQuizbowl(level) => {
                self.quizbowl_level = level;
                self.quizbowl = None;
                self.quizbowl_bonus = None;
                self.recent_qb_categories.clear();
                self.start_quizbowl(ctx.clone());
                self.screen = Screen::Quizbowl;
            }
            LobbyAction::StartJeopardy => {
                {
                    let phone_players = self.server.as_ref().map(|s| s.players()).unwrap_or_default();
                    let config = if phone_players.is_empty() {
                        JeopardyConfig { mode: JeopardyMode::Solo, team_count: 1, team_names: vec![] }
                    } else {
                        JeopardyConfig {
                            mode: JeopardyMode::Teams,
                            team_count: phone_players.len() as u8,
                            team_names: phone_players.iter().map(|p| p.name.clone()).collect(),
                        }
                    };
                    self.mp.jeopardy.player_ids = phone_players.iter().map(|p| p.id).collect();
                    self.start_jeopardy(config);
                    self.screen = Screen::Jeopardy;
                }
            }
            LobbyAction::StartMix(level) => {
                self.start_mix(ctx.clone(), level);
                self.screen = Screen::Mix;
            }
        }
    }

    fn update_mix(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        let dt = ctx.input(|i| i.stable_dt);
        let signal = self.mix.as_mut().and_then(|m| m.show(ui, dt));
        match signal {
            Some(MixSignal::Quit) => {
                self.screen = Screen::Home;
                self.mix = None;
            }
            Some(MixSignal::Restart) => {
                self.mp.new_game(self.server.as_deref());
                let level = self.mix_level;
                self.start_mix(ctx.clone(), level);
            }
            Some(MixSignal::NeedQb { exclude_category }) => {
                self.fetch_mix_qb(ctx.clone(), exclude_category);
            }
            None => {}
        }
    }

    // ── Multiplayer tick: broadcast state + drain phone inputs ────────────────

    fn tick_multiplayer(&mut self, ctx: &Context) {
        let Some(server) = self.server.clone() else { return };
        let now = ctx.input(|i| i.time);

        // Leaving a game for Home/Lobby: park phones on a waiting screen so they
        // don't sit on a stale question.
        if self.mp.last_screen.as_ref() != Some(&self.screen) {
            let was_game = matches!(
                self.mp.last_screen,
                Some(Screen::Trivia | Screen::Quizbowl | Screen::Jeopardy | Screen::Mix)
            );
            if was_game && matches!(self.screen, Screen::Home | Screen::Lobby) {
                server.broadcast(&ServerMsg::Waiting { msg: "Round over — waiting for the host".into() });
            }
            self.mp.last_screen = Some(self.screen.clone());
        }

        match self.screen {
            Screen::Trivia => self.mp.trivia.tick(&server, self.trivia.as_ref()),
            Screen::Quizbowl if self.quizbowl_bonus.is_some() => {
                self.mp.park(&server, "Bonus round — watch the TV");
            }
            Screen::Quizbowl => {
                self.mp.unpark();
                self.mp.quizbowl.tick(&server, self.quizbowl.as_mut(), now);
            }
            Screen::Jeopardy => self.mp_tick_jeopardy(&server, now),
            Screen::Mix => {
                let Some(mix) = self.mix.as_mut() else { return multiplayer::drain(&server) };
                if matches!(mix.phase, MixPhase::Trivia | MixPhase::Quizbowl | MixPhase::JeopardyPhase) {
                    self.mp.unpark();
                }
                match mix.phase {
                    MixPhase::Trivia => self.mp.trivia.tick(&server, mix.trivia.as_ref()),
                    MixPhase::Quizbowl => self.mp.quizbowl.tick(&server, mix.quizbowl.as_mut(), now),
                    MixPhase::JeopardyPhase => self.mp.mix_clue.tick(&server, mix.current_clue_mut(), now),
                    MixPhase::Summary => self.mp.park(&server, "Mix complete — scores are on the TV"),
                    MixPhase::Loading | MixPhase::QuizbowlLoading => {
                        self.mp.park(&server, "Loading the next round…")
                    }
                }
            }
            _ => multiplayer::drain(&server),
        }
    }

    fn mp_tick_jeopardy(&mut self, server: &GameServer, now: f64) {
        use crate::screens::jeopardy::clue::ClueState;
        use crate::screens::jeopardy::final_jeopardy::FinalPhase;

        let Some(board) = self.jeopardy_board.as_ref() else { return multiplayer::drain(server) };
        let js = &mut self.mp.jeopardy;

        // The board is the score ledger; mirror it onto the phone seats.
        for (i, &id) in js.player_ids.iter().enumerate() {
            if let Some((_, score)) = board.players.get(i) {
                server.set_score(id, *score);
            }
        }
        let active_id = js.player_ids.get(board.active).copied();
        let active_name = board.players.get(board.active).map(|(n, _)| n.clone()).unwrap_or_default();

        if self.jeopardy_over.is_some() {
            if !js.over_sent {
                js.over_sent = true;
                let winner = board.players.iter().max_by_key(|p| p.1).map(|p| p.0.clone()).unwrap_or_default();
                server.broadcast(&ServerMsg::Waiting { msg: format!("{winner} wins! Final standings are on the TV") });
            }
            return multiplayer::drain(server);
        }

        match &self.jeopardy_overlay {
            None => {
                if !js.board_sent {
                    js.board_sent = true;
                    js.clue_sent = false;
                    js.dd_clue_sent = false;
                    js.result_sent = false;
                    js.final_clue_sent = false;
                    let categories: Vec<String> = board.categories.iter().map(|(n, _)| n.clone()).collect();
                    let values: Vec<Vec<i32>> = board.categories.iter()
                        .map(|(_, clues)| clues.iter().map(|c| c.value).collect())
                        .collect();
                    let used: Vec<Vec<bool>> = board.categories.iter().enumerate()
                        .map(|(c, (_, clues))| (0..clues.len()).map(|r| board.used[c][r]).collect())
                        .collect();
                    let scores = board.players.clone();
                    server.send_each(|id| ServerMsg::JeopardyBoard {
                        categories: categories.clone(),
                        values: values.clone(),
                        used: used.clone(),
                        scores: scores.clone(),
                        active_player: active_name.clone(),
                        is_your_turn: Some(id) == active_id,
                    });
                }
            }
            Some(overlay) => {
                js.board_sent = false;
                // The overlay just opened.
                if !js.clue_sent {
                    js.clue_sent = true;
                    match overlay {
                        JeopardyOverlay::Clue(cs) => server.send_each(|id| ServerMsg::JeopardyClue {
                            category: cs.clue.category.clone(),
                            value: cs.clue.value,
                            text: cs.clue.question.clone(),
                            is_you_answer: Some(id) == active_id,
                            active_player: active_name.clone(),
                        }),
                        JeopardyOverlay::DailyDouble(dd) => server.send_each(|id| ServerMsg::JeopardyDd {
                            category: dd.clue.category.clone(),
                            value: dd.clue.value,
                            max_wager: dd.max_wager(),
                            is_you_answer: Some(id) == active_id,
                            active_player: active_name.clone(),
                        }),
                        JeopardyOverlay::Final(f) => server.send_each(|id| {
                            let seat = js.seat(id);
                            ServerMsg::JeopardyFinalWager {
                                category: f.clue.category.clone(),
                                max_wager: seat.map(|i| f.max_wager(i)).unwrap_or(0),
                                can_play: seat.is_some_and(|i| f.can_play(i)),
                            }
                        }),
                    }
                }
                match overlay {
                    // Daily Double: once the wager is locked, show the clue to everyone.
                    JeopardyOverlay::DailyDouble(dd) if dd.wager.is_some() && !js.dd_clue_sent => {
                        js.dd_clue_sent = true;
                        server.send_each(|id| ServerMsg::JeopardyClue {
                            category: dd.clue.category.clone(),
                            value: dd.wager.unwrap_or(0),
                            text: dd.clue.question.clone(),
                            is_you_answer: Some(id) == active_id,
                            active_player: active_name.clone(),
                        });
                    }
                    JeopardyOverlay::Final(f) if f.phase != FinalPhase::Wager && !js.final_clue_sent => {
                        js.final_clue_sent = true;
                        server.send_each(|id| ServerMsg::JeopardyFinalClue {
                            category: f.clue.category.clone(),
                            text: f.clue.question.clone(),
                            can_answer: js.seat(id).is_some_and(|i| f.can_play(i)),
                        });
                    }
                    _ => {}
                }
                // Verdicts, however they were reached (phone, typed, or host-judged).
                if !js.result_sent {
                    match overlay {
                        JeopardyOverlay::Clue(cs) => {
                            if let ClueState::Answered { correct } = cs.state {
                                js.result_sent = true;
                                let delta = if correct { cs.clue.value } else { -cs.clue.value };
                                server.broadcast(&ServerMsg::JeopardyResult { correct, answer: cs.clue.answer.clone(), delta });
                            }
                        }
                        JeopardyOverlay::DailyDouble(dd) => {
                            if let Some(correct) = dd.answered {
                                js.result_sent = true;
                                let w = dd.wager.unwrap_or(0);
                                let delta = if correct { w } else { -w };
                                server.broadcast(&ServerMsg::JeopardyResult { correct, answer: dd.clue.answer.clone(), delta });
                            }
                        }
                        JeopardyOverlay::Final(f) if f.phase == FinalPhase::Result => {
                            js.result_sent = true;
                            server.send_each(|id| match js.seat(id) {
                                Some(i) if f.can_play(i) => ServerMsg::JeopardyResult {
                                    correct: matches!(f.responses[i], Some((_, true))),
                                    answer: f.clue.answer.clone(),
                                    delta: f.delta(i),
                                },
                                _ => ServerMsg::Waiting { msg: format!("The answer: {}", f.clue.answer) },
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        // Phone inputs. Final takes every seat; otherwise only the active seat may act.
        while let Some(input) = server.pop_input() {
            if let Some(JeopardyOverlay::Final(f)) = self.jeopardy_overlay.as_mut() {
                if let Some(seat) = self.mp.jeopardy.seat(input.player_id) {
                    match input.msg {
                        ClientMsg::Wager { amount } => f.phone_wager(seat, amount),
                        ClientMsg::AnswerText { text } => f.phone_answer(seat, &text),
                        _ => {}
                    }
                }
                continue;
            }
            if Some(input.player_id) != active_id {
                continue;
            }
            match input.msg {
                ClientMsg::PickClue { col, row } if self.jeopardy_overlay.is_none() => {
                    self.open_clue(col, row);
                }
                ClientMsg::Wager { amount } => {
                    if let Some(JeopardyOverlay::DailyDouble(dd)) = self.jeopardy_overlay.as_mut() {
                        dd.set_phone_wager(amount);
                    }
                }
                ClientMsg::AnswerText { text } => match self.jeopardy_overlay.as_mut() {
                    Some(JeopardyOverlay::Clue(cs)) => cs.set_phone_answer(&text, now),
                    Some(JeopardyOverlay::DailyDouble(dd)) => {
                        dd.set_phone_answer(&text, now);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

/// Debug-only: `IT_SHOT=out.png` saves a screenshot ~1.5s after launch and quits,
/// so UI work can be checked without clicking through (pair with IT_DEV/IT_SCREEN).
#[cfg(debug_assertions)]
fn dev_screenshot(ctx: &Context) {
    let Ok(path) = std::env::var("IT_SHOT") else { return };
    let delay: f64 = std::env::var("IT_SHOT_AT").ok().and_then(|s| s.parse().ok()).unwrap_or(1.5);
    let requested = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("dev_shot_req"))).unwrap_or(false);
    if !requested && ctx.input(|i| i.time) > delay {
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("dev_shot_req"), true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
    }
    let shot = ctx.input(|i| {
        i.raw.events.iter().find_map(|e| match e {
            egui::Event::Screenshot { image, .. } => Some(image.clone()),
            _ => None,
        })
    });
    if let Some(img) = shot {
        let [w, h] = img.size;
        let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
        if let Some(buf) = image::RgbaImage::from_raw(w as u32, h as u32, bytes) {
            let _ = buf.save(&path);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
