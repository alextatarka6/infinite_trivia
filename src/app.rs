use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use egui::Context;
use crate::api::{jeopardy::JeopardyStore, opentdb, qbreader};
use crate::game::state::Screen;
use crate::screens::{
    home::HomeScreen,
    jeopardy::{
        board::{Board, BoardAction},
        clue::ClueScreen,
        daily_double::DailyDoubleScreen,
        final_jeopardy::FinalJeopardy,
        setup::{JeopardyConfig, JeopardyMode, JeopardySetupScreen, SetupAction},
    },
    trivia::question::TriviaScreen,
    quizbowl::tossup::TossupScreen,
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
    jeopardy_store: Option<JeopardyStore>,
    jeopardy_store_error: Option<String>,
    jeopardy_setup: Option<JeopardySetupScreen>,
    jeopardy_board: Option<Board>,
    jeopardy_overlay: Option<JeopardyOverlay>,
    daily_doubles: Vec<(usize, usize)>,
    trivia: Option<TriviaScreen>,
    quizbowl: Option<TossupScreen>,
    quizbowl_level: u8,
    recent_qb_categories: VecDeque<String>,
    rt: tokio::runtime::Runtime,
    pending_trivia: Arc<Mutex<Option<Result<Vec<crate::api::opentdb::TriviaQuestion>, String>>>>,
    pending_qb: Arc<Mutex<Option<Result<crate::api::qbreader::Tossup, String>>>>,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (jeopardy_store, jeopardy_store_error) = match JeopardyStore::load(JSON_PATH) {
            Ok(store) => (Some(store), None),
            Err(e) => (None, Some(e)),
        };

        Self {
            screen: Screen::Home,
            home_screen: HomeScreen::default(),
            jeopardy_store,
            jeopardy_store_error,
            jeopardy_setup: None,
            jeopardy_board: None,
            jeopardy_overlay: None,
            daily_doubles: vec![],
            trivia: None,
            quizbowl: None,
            quizbowl_level: 2,
            recent_qb_categories: VecDeque::new(),
            rt: tokio::runtime::Runtime::new().expect("tokio runtime"),
            pending_trivia: Arc::new(Mutex::new(None)),
            pending_qb: Arc::new(Mutex::new(None)),
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
                                self.recent_qb_categories.clear();
                                self.start_quizbowl(ctx.clone());
                            }
                            Screen::Jeopardy | Screen::Home => {}
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
                                TriviaAction::Next { .. } => {
                                    if trivia.current_idx >= trivia.questions.len() {
                                        self.screen = Screen::Home;
                                    }
                                }
                            }
                        }
                    }
                }

                Screen::Quizbowl => {
                    self.update_quizbowl(ui, ctx);
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
        use crate::screens::quizbowl::tossup::TossupAction;
        let dt = ctx.input(|i| i.stable_dt);
        let action = self.quizbowl.as_mut().and_then(|qb| qb.show(ui, dt));
        match action {
            Some(TossupAction::Next) => {
                if let Some(ref qb) = self.quizbowl {
                    let cat = qb.tossup.category.clone();
                    if !cat.is_empty() {
                        self.recent_qb_categories.push_back(cat);
                        if self.recent_qb_categories.len() > 4 {
                            self.recent_qb_categories.pop_front();
                        }
                    }
                }
                self.start_quizbowl(ctx.clone());
            }
            Some(TossupAction::Quit) => {
                self.screen = Screen::Home;
                self.quizbowl = None;
            }
            None => {}
        }
    }
}
