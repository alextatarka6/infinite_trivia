use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use egui::Context;
use crate::api::{jeopardy::JeopardyStore, opentdb, qbreader};
use crate::game::state::Screen;
use crate::screens::{
    dev::DevMenu,
    home::HomeScreen,
    jeopardy::{
        board::{Board, BoardAction},
        clue::ClueScreen,
        daily_double::DailyDoubleScreen,
        final_jeopardy::FinalJeopardy,
        setup::{JeopardyConfig, JeopardyMode, JeopardySetupScreen, SetupAction},
    },
    trivia::question::TriviaScreen,
    quizbowl::{bonus::BonusScreen, tossup::TossupScreen},
};
use crate::theme;

const JSON_PATH: &str = "assets/JEOPARDY_QUESTIONS1.json";

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
    jeopardy_setup: Option<JeopardySetupScreen>,
    jeopardy_board: Option<Board>,
    jeopardy_overlay: Option<JeopardyOverlay>,
    daily_doubles: Vec<(usize, usize)>,
    trivia: Option<TriviaScreen>,
    trivia_level: u8,
    quizbowl: Option<TossupScreen>,
    quizbowl_bonus: Option<BonusScreen>,
    quizbowl_level: u8,
    recent_qb_categories: VecDeque<String>,
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

            // Helper to register a font file into fonts.font_data
            let load = |fonts: &mut egui::FontDefinitions, key: &str, path: &str| {
                if let Ok(bytes) = std::fs::read(path) {
                    fonts.font_data.insert(key.to_owned(), egui::FontData::from_owned(bytes));
                }
            };

            load(&mut fonts, "spectral-400",        "assets/spectral-400.ttf");
            load(&mut fonts, "spectral-500",        "assets/spectral-500.ttf");
            load(&mut fonts, "spectral-600",        "assets/spectral-600.ttf");
            load(&mut fonts, "spectral-700",        "assets/spectral-700.ttf");
            load(&mut fonts, "spectral-500italic",  "assets/spectral-500italic.ttf");
            load(&mut fonts, "plexsans-400",        "assets/ibmplexsans-400.ttf");
            load(&mut fonts, "plexsans-500",        "assets/ibmplexsans-500.ttf");
            load(&mut fonts, "plexsans-600",        "assets/ibmplexsans-600.ttf");
            load(&mut fonts, "plexmono-400",        "assets/ibmplexmono-400.ttf");
            load(&mut fonts, "plexmono-500",        "assets/ibmplexmono-500.ttf");
            load(&mut fonts, "plexmono-600",        "assets/ibmplexmono-600.ttf");

            // IBM Plex Sans replaces the default proportional family
            fonts.families.entry(egui::FontFamily::Proportional).or_default()
                .splice(0..0, ["plexsans-400".to_owned()]);

            // IBM Plex Mono replaces the default monospace family
            fonts.families.entry(egui::FontFamily::Monospace).or_default()
                .splice(0..0, ["plexmono-400".to_owned()]);

            // Named families for weight/style variants
            let named = &mut fonts.families;
            named.insert(egui::FontFamily::Name("spectral".into()),         vec!["spectral-700".to_owned()]);
            named.insert(egui::FontFamily::Name("spectral-medium".into()),  vec!["spectral-500".to_owned()]);
            named.insert(egui::FontFamily::Name("spectral-regular".into()), vec!["spectral-400".to_owned()]);
            named.insert(egui::FontFamily::Name("spectral-italic".into()),  vec!["spectral-500italic".to_owned()]);
            named.insert(egui::FontFamily::Name("plex-medium".into()),      vec!["plexsans-500".to_owned()]);
            named.insert(egui::FontFamily::Name("plex-semibold".into()),    vec!["plexsans-600".to_owned()]);
            named.insert(egui::FontFamily::Name("mono-medium".into()),      vec!["plexmono-500".to_owned()]);
            named.insert(egui::FontFamily::Name("mono-semibold".into()),    vec!["plexmono-600".to_owned()]);

            // Keep old "serif" alias so existing call-sites don't break
            named.insert(egui::FontFamily::Name("serif".into()), vec!["spectral-700".to_owned()]);

            cc.egui_ctx.set_fonts(fonts);
        }

        let (jeopardy_store, jeopardy_store_error) = match JeopardyStore::load(JSON_PATH) {
            Ok(store) => (Some(store), None),
            Err(e) => (None, Some(e)),
        };

        Self {
            screen: if std::env::var("IT_DEV").is_ok() {
                Screen::DevMenu
            } else {
                Screen::Home
            },
            home_screen: HomeScreen::default(),
            dev_menu: DevMenu::default(),
            jeopardy_store,
            jeopardy_store_error,
            jeopardy_setup: None,
            jeopardy_board: None,
            jeopardy_overlay: None,
            daily_doubles: vec![],
            trivia: None,
            trivia_level: 2,
            quizbowl: None,
            quizbowl_bonus: None,
            quizbowl_level: 2,
            recent_qb_categories: VecDeque::new(),
            rt: tokio::runtime::Runtime::new().expect("tokio runtime"),
            pending_trivia: Arc::new(Mutex::new(None)),
            pending_qb: Arc::new(Mutex::new(None)),
            pending_bonus: Arc::new(Mutex::new(None)),
        }
    }

    fn start_jeopardy(&mut self, config: JeopardyConfig) {
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

    fn pick_daily_doubles(cols: usize, rows: usize, count: usize) -> Vec<(usize, usize)> {
        use std::collections::HashSet;
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as usize;
        let mut positions = HashSet::new();
        let mut i = 0usize;
        while positions.len() < count {
            let col = (seed.wrapping_add(i * 7)) % cols;
            let row = 1 + (seed.wrapping_add(i * 13)) % (rows - 1).max(1);
            positions.insert((col, row));
            i += 1;
        }
        positions.into_iter().collect()
    }

    fn start_trivia(&mut self, ctx: Context, level: u8) {
        self.trivia_level = level;
        self.trivia = Some(TriviaScreen::loading());
        let pending = Arc::clone(&self.pending_trivia);
        let diff = match level {
            1 => "easy",
            3 => "hard",
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

        // Poll async trivia result
        if let Ok(mut guard) = self.pending_trivia.try_lock() {
            if let Some(result) = guard.take() {
                self.trivia = Some(match result {
                    Ok(qs) => TriviaScreen::new(qs),
                    Err(e) => TriviaScreen::with_error(e),
                });
            }
        }

        // Poll async quizbowl result
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

        // Poll async bonus result
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

        let dt = ctx.input(|i| i.stable_dt);

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(theme::BLUE_BG))
            .show(ctx, |ui| match self.screen {
                Screen::Home => {
                    if let Some((next, level)) = self.home_screen.show(ui) {
                        self.screen = next.clone();
                        match next {
                            Screen::JeopardySetup => {
                                self.jeopardy_setup = Some(JeopardySetupScreen::default());
                            }
                            Screen::Trivia => self.start_trivia(ctx.clone(), level),
                            Screen::Quizbowl => {
                                self.quizbowl_level = level;
                                self.quizbowl = None;
                                self.quizbowl_bonus = None;
                                self.recent_qb_categories.clear();
                                self.start_quizbowl(ctx.clone());
                            }
                            Screen::Jeopardy | Screen::Home | Screen::DevMenu => {}
                        }
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
                    if let Some(ref mut trivia) = self.trivia {
                        use crate::screens::trivia::question::TriviaAction;
                        if let Some(action) = trivia.show(ui, dt) {
                            match action {
                                TriviaAction::Quit => self.screen = Screen::Home,
                                TriviaAction::Restart => {
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

                Screen::DevMenu => {
                    if self.dev_menu.show(ui, dt) {
                        self.screen = Screen::Home;
                    }
                }
            });

        ctx.request_repaint();
    }
}

impl App {
    fn update_jeopardy(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        if let Some(err) = &self.jeopardy_store_error {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(
                    egui::RichText::new("Could not load Jeopardy data")
                        .font(theme::subheading_font())
                        .color(theme::RED_WRONG),
                );
                ui.label(egui::RichText::new(err).color(theme::WHITE));
                ui.add_space(12.0);
                if ui.button("Back to Menu").clicked() {
                    self.screen = Screen::Home;
                }
            });
            return;
        }

        let overlay_done = match self.jeopardy_overlay.take() {
            Some(JeopardyOverlay::Clue(mut s)) => {
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
                    Some(FinalAction::Done { score_delta }) => {
                        if let Some(board) = &mut self.jeopardy_board {
                            board.players[board.active].1 += score_delta;
                        }
                        self.screen = Screen::Home;
                        self.jeopardy_board = None;
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
            if let Some(board) = self.jeopardy_board.as_mut() {
                let daily_doubles = self.daily_doubles.clone();
                let action = board.show(ui);

                match action {
                    Some(BoardAction::SelectClue { col, row }) => {
                        let is_dd = daily_doubles.contains(&(col, row));
                        if let Some((_, clues)) = board.categories.get(col) {
                            if let Some(clue) = clues.get(row).cloned() {
                                board.used[col][row] = true;
                                let active_score = board.active_score();
                                if is_dd {
                                    self.jeopardy_overlay = Some(JeopardyOverlay::DailyDouble(
                                        DailyDoubleScreen::new(clue, active_score),
                                    ));
                                } else {
                                    self.jeopardy_overlay =
                                        Some(JeopardyOverlay::Clue(ClueScreen::new(clue)));
                                }
                            }
                        }
                    }

                    Some(BoardAction::GoToFinal) => {
                        if board.is_double_jeopardy {
                            if let Some(store) = &self.jeopardy_store {
                                if let Some(clue) = store.random_final() {
                                    let active_score = board.active_score();
                                    self.jeopardy_overlay = Some(JeopardyOverlay::Final(
                                        FinalJeopardy::new(clue, active_score),
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
}
