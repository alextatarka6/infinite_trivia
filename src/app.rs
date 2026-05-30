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
    },
    trivia::question::TriviaScreen,
    quizbowl::tossup::TossupScreen,
};
use crate::theme;

const JSON_PATH: &str = "assets/JEOPARDY_QUESTIONS1.json";

// Overlays shown on top of the board; the board is kept alive separately.
enum JeopardyOverlay {
    Clue(ClueScreen),
    DailyDouble(DailyDoubleScreen),
    Final(FinalJeopardy),
}

pub struct App {
    screen: Screen,
    jeopardy_store: Option<JeopardyStore>,
    jeopardy_store_error: Option<String>,
    // Board persists independently; overlay is swapped in/out
    jeopardy_board: Option<Board>,
    jeopardy_overlay: Option<JeopardyOverlay>,
    daily_doubles: Vec<(usize, usize)>,
    // Trivia & Quizbowl
    trivia: Option<TriviaScreen>,
    quizbowl: Option<TossupScreen>,
    // Async runtime + shared result slots
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
            jeopardy_store,
            jeopardy_store_error,
            jeopardy_board: None,
            jeopardy_overlay: None,
            daily_doubles: vec![],
            trivia: None,
            quizbowl: None,
            rt: tokio::runtime::Runtime::new().expect("tokio runtime"),
            pending_trivia: Arc::new(Mutex::new(None)),
            pending_qb: Arc::new(Mutex::new(None)),
        }
    }

    fn start_jeopardy(&mut self) {
        let store = match &self.jeopardy_store {
            Some(s) => s,
            None => return,
        };
        match store.random_board("Jeopardy!") {
            Some(cats) => {
                let n_cols = cats.len();
                let n_rows = cats.first().map(|(_, v)| v.len()).unwrap_or(5);
                self.daily_doubles = Self::pick_daily_doubles(n_cols, n_rows, 1);
                self.jeopardy_board = Some(Board::new(cats, false));
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

    fn start_trivia(&mut self, ctx: Context) {
        self.trivia = Some(TriviaScreen::loading());
        let pending = Arc::clone(&self.pending_trivia);
        self.rt.spawn(async move {
            let result = opentdb::fetch_questions(10).await;
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
        self.rt.spawn(async move {
            let result = qbreader::fetch_tossup().await;
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
                    if let Some(next) = HomeScreen::show(ui) {
                        self.screen = next.clone();
                        match next {
                            Screen::Jeopardy => self.start_jeopardy(),
                            Screen::Trivia => self.start_trivia(ctx.clone()),
                            Screen::Quizbowl => self.start_quizbowl(ctx.clone()),
                            Screen::Home => {}
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
        // Show store load error if any
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

        // Handle active overlay first; board renders only when no overlay is active.
        let overlay_done = match self.jeopardy_overlay.take() {
            Some(JeopardyOverlay::Clue(mut s)) => {
                use crate::screens::jeopardy::clue::ClueAction;
                match s.show(ui) {
                    Some(ClueAction::Done { correct }) => {
                        if let Some(board) = &mut self.jeopardy_board {
                            board.score += if correct { s.clue.value } else { -s.clue.value };
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
                            board.score += winnings;
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
                            board.score += score_delta;
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
                                if is_dd {
                                    self.jeopardy_overlay =
                                        Some(JeopardyOverlay::DailyDouble(
                                            DailyDoubleScreen::new(clue, board.score),
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
                                    self.jeopardy_overlay = Some(JeopardyOverlay::Final(
                                        FinalJeopardy::new(clue, board.score),
                                    ));
                                }
                            }
                        } else {
                            let prev_score = board.score;
                            if let Some(store) = &self.jeopardy_store {
                                if let Some(cats) = store.random_board("Double Jeopardy!") {
                                    let n_cols = cats.len();
                                    let n_rows =
                                        cats.first().map(|(_, v)| v.len()).unwrap_or(5);
                                    self.daily_doubles =
                                        Self::pick_daily_doubles(n_cols, n_rows, 2);
                                    let mut new_board = Board::new(cats, true);
                                    new_board.score = prev_score;
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
        if let Some(ref mut qb) = self.quizbowl {
            match qb.show(ui, dt) {
                Some(TossupAction::Next) => self.start_quizbowl(ctx.clone()),
                Some(TossupAction::Quit) => {
                    self.screen = Screen::Home;
                    self.quizbowl = None;
                }
                None => {}
            }
        }
    }
}
