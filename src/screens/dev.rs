//! Dev-only screen gallery. Jumps straight into any in-game screen with mock
//! data so the UI can be evaluated without the API or playing through.
//! Enabled by launching with the env var `IT_DEV=1`.

use egui::{Align2, Pos2, Rect, RichText, Sense, Vec2};

use crate::api::jeopardy::Clue;
use crate::api::opentdb::TriviaQuestion;
use crate::api::qbreader::{Bonus, BonusPart, Tossup};
use crate::screens::quizbowl::bonus::BonusScreen;
use crate::screens::jeopardy::board::{Board, BoardAction};
use crate::screens::jeopardy::clue::{ClueScreen, ClueState};
use crate::screens::jeopardy::daily_double::DailyDoubleScreen;
use crate::screens::jeopardy::final_jeopardy::FinalJeopardy;
use crate::screens::jeopardy::setup::{JeopardySetupScreen, SetupAction};
use crate::screens::quizbowl::tossup::{TossupAction, TossupScreen, TossupState};
use crate::screens::trivia::question::{TriviaAction, TriviaScreen};
use crate::theme::{self, BLUE_MID, GOLD, INK_DIM, LINE, WHITE};

/// A screen currently being previewed in the gallery.
pub enum DevView {
    Trivia(TriviaScreen),
    Quizbowl(TossupScreen),
    Bonus(BonusScreen),
    Setup(JeopardySetupScreen),
    Board(Board),
    Clue(ClueScreen),
    DailyDouble(DailyDoubleScreen),
    Final(FinalJeopardy),
}

pub struct DevMenu {
    pub view: Option<DevView>,
}

impl Default for DevMenu {
    fn default() -> Self {
        // `IT_SCREEN=<key>` opens a screen directly (skips the gallery click).
        let view = std::env::var("IT_SCREEN").ok().and_then(|k| match k.as_str() {
            "trivia" => Some(DevView::Trivia(mock_trivia())),
            "trivia-summary" => Some(DevView::Trivia(mock_trivia_summary())),
            "trivia-loading" => Some(DevView::Trivia(TriviaScreen::loading())),
            "qb" | "qb-reading" => Some(DevView::Quizbowl(mock_qb(QbPhase::Reading))),
            "qb-buzzed" => Some(DevView::Quizbowl(mock_qb(QbPhase::Buzzed))),
            "qb-correct" => Some(DevView::Quizbowl(mock_qb(QbPhase::Correct))),
            "qb-incorrect" => Some(DevView::Quizbowl(mock_qb(QbPhase::Incorrect))),
            "qb-timeout" => Some(DevView::Quizbowl(mock_qb(QbPhase::Timeout))),
            "bonus" => Some(DevView::Bonus(BonusScreen::new(mock_bonus(), 55, 3))),
            "setup" => Some(DevView::Setup(JeopardySetupScreen::default())),
            "setup-teams" => Some(DevView::Setup(JeopardySetupScreen::dev_teams())),
            "board" => Some(DevView::Board(mock_board())),
            "clue" => Some(DevView::Clue(ClueScreen::new(mock_clue()))),
            "clue-result" => {
                let mut c = ClueScreen::new(mock_clue());
                c.state = ClueState::Answered { correct: true };
                Some(DevView::Clue(c))
            }
            "dd" => Some(DevView::DailyDouble(DailyDoubleScreen::new(mock_clue(), 2400))),
            "dd-result" => {
                let mut d = DailyDoubleScreen::new(mock_clue(), 2400);
                d.wager = Some(1200);
                d.answered = Some(false);
                Some(DevView::DailyDouble(d))
            }
            "final" => Some(DevView::Final(FinalJeopardy::new(mock_clue(), 5200))),
            "final-result" => {
                let mut f = FinalJeopardy::new(mock_clue(), 5200);
                f.show_category = false;
                f.wager = Some(2000);
                f.answered = Some(true);
                Some(DevView::Final(f))
            }
            _ => None,
        });
        Self { view }
    }
}

impl DevMenu {
    /// Returns `Some(true)` to request the real Home screen, otherwise drives the
    /// gallery. `dt` is forwarded to time-based screens (trivia/quizbowl).
    pub fn show(&mut self, ui: &mut egui::Ui, dt: f32) -> bool {
        // Render an active preview, intercepting its quit/done actions to return
        // to the gallery instead of the real Home screen.
        if let Some(view) = &mut self.view {
            let back = match view {
                DevView::Trivia(s) => matches!(
                    s.show(ui, dt),
                    Some(TriviaAction::Quit) | Some(TriviaAction::Restart)
                ),
                DevView::Quizbowl(s) => matches!(s.show(ui, dt), Some(TossupAction::Quit)),
                DevView::Bonus(s) => s.show(ui).is_some(),
                DevView::Setup(s) => matches!(s.show(ui), Some(SetupAction::Back) | Some(SetupAction::Start(_))),
                DevView::Board(b) => match b.show(ui) {
                    Some(BoardAction::Quit) => true,
                    Some(BoardAction::SelectClue { col, row }) => {
                        b.used[col][row] = true; // spend the tile, stay on board
                        false
                    }
                    _ => false,
                },
                DevView::Clue(s) => s.show(ui).is_some(),
                DevView::DailyDouble(s) => s.show(ui).is_some(),
                DevView::Final(s) => s.show(ui).is_some(),
            };
            if back {
                self.view = None;
            }
            // tiny floating "← gallery" affordance, top-left
            if dev_back_button(ui) {
                self.view = None;
            }
            return false;
        }

        // -- The gallery grid -------------------------------------------------
        crate::screens::kit::ambient(ui.ctx());
        let mut go_home = false;

        let avail = ui.available_size();
        ui.add_space((avail.y - 430.0).max(16.0) / 2.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("DEV · SCREEN GALLERY")
                    .font(theme::mono(13.0))
                    .color(GOLD),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new("jump into any screen with mock data")
                    .font(theme::mono(11.0))
                    .color(INK_DIM),
            );
            ui.add_space(24.0);

            let buttons: &[(&str, fn() -> DevView)] = &[
                ("Trivia · question", || DevView::Trivia(mock_trivia())),
                ("Trivia · summary", || DevView::Trivia(mock_trivia_summary())),
                ("Trivia · loading", || DevView::Trivia(TriviaScreen::loading())),
                ("Quizbowl · reading", || DevView::Quizbowl(mock_qb(QbPhase::Reading))),
                ("Quizbowl · buzzed", || DevView::Quizbowl(mock_qb(QbPhase::Buzzed))),
                ("Quizbowl · correct", || DevView::Quizbowl(mock_qb(QbPhase::Correct))),
                ("Quizbowl · incorrect", || DevView::Quizbowl(mock_qb(QbPhase::Incorrect))),
                ("Quizbowl · timeout", || DevView::Quizbowl(mock_qb(QbPhase::Timeout))),
                ("Quizbowl · loading", || DevView::Quizbowl(TossupScreen::loading(0, 0))),
                ("Quizbowl · bonus", || DevView::Bonus(BonusScreen::new(mock_bonus(), 55, 3))),
                ("Jeopardy · setup", || DevView::Setup(JeopardySetupScreen::default())),
                ("Jeopardy · board", || DevView::Board(mock_board())),
                ("Jeopardy · clue", || DevView::Clue(ClueScreen::new(mock_clue()))),
                ("Jeopardy · daily double", || DevView::DailyDouble(DailyDoubleScreen::new(mock_clue(), 2400))),
                ("Jeopardy · final", || DevView::Final(FinalJeopardy::new(mock_clue(), 5200))),
            ];

            // 3-column grid of pill buttons
            let cols = 3;
            let bw = 230.0;
            let bh = 46.0;
            let gap = 14.0;
            let row_w = cols as f32 * bw + (cols as f32 - 1.0) * gap;

            for chunk in buttons.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.add_space((avail.x - row_w) / 2.0);
                    for (i, (label, make)) in chunk.iter().enumerate() {
                        if i > 0 {
                            ui.add_space(gap);
                        }
                        if pill(ui, label, Vec2::new(bw, bh)) {
                            self.view = Some(make());
                        }
                    }
                });
                ui.add_space(gap);
            }

            ui.add_space(18.0);
            if pill(ui, "→ real Home screen", Vec2::new(230.0, 42.0)) {
                go_home = true;
            }
        });

        go_home
    }
}

// ---------------------------------------------------------------------------
// Simple gallery button
// ---------------------------------------------------------------------------

fn pill(ui: &mut egui::Ui, label: &str, size: Vec2) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let t = ui.ctx().animate_bool(resp.id, resp.hovered());
    let fill = crate::screens::kit::lerp_color(BLUE_MID, GOLD, t);
    let border = crate::screens::kit::lerp_color(LINE, GOLD, t);
    let text = crate::screens::kit::lerp_color(WHITE, theme::ON_ACCENT, t);
    ui.painter().rect_filled(rect, egui::Rounding::same(12.0), fill);
    ui.painter()
        .rect_stroke(rect, egui::Rounding::same(12.0), egui::Stroke::new(1.0, border));
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, label, theme::plex_medium(14.0), text);
    resp.clicked()
}

fn dev_back_button(ui: &mut egui::Ui) -> bool {
    let screen = ui.ctx().screen_rect();
    let rect = Rect::from_min_size(Pos2::new(screen.left() + 16.0, screen.top() + 16.0), Vec2::new(110.0, 30.0));
    let resp = ui.interact(rect, egui::Id::new("dev_back"), Sense::click());
    let t = ui.ctx().animate_bool(resp.id, resp.hovered());
    ui.painter().rect_stroke(
        rect,
        egui::Rounding::same(999.0),
        egui::Stroke::new(1.0, crate::screens::kit::lerp_color(LINE, INK_DIM, t)),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "← gallery",
        theme::plex_medium(12.0),
        crate::screens::kit::lerp_color(INK_DIM, WHITE, t),
    );
    resp.clicked()
}

// ---------------------------------------------------------------------------
// Mock data
// ---------------------------------------------------------------------------

fn mock_trivia() -> TriviaScreen {
    TriviaScreen::new(mock_questions())
}

fn mock_trivia_summary() -> TriviaScreen {
    let mut s = TriviaScreen::new(mock_questions());
    s.current_idx = s.questions.len();
    s.score = 740;
    s.correct_count = 7;
    s.best_streak = 4;
    s
}

fn mock_questions() -> Vec<TriviaQuestion> {
    vec![
        TriviaQuestion {
            category: "Science: Computers".into(),
            question: "In computing, what does the acronym 'RAM' stand for?".into(),
            correct_answer: "Random Access Memory".into(),
            incorrect_answers: vec![
                "Rapid Action Module".into(),
                "Read Algorithm Memory".into(),
                "Runtime Allocation Map".into(),
            ],
            difficulty: "medium".into(),
        },
        TriviaQuestion {
            category: "Geography".into(),
            question: "Which capital city sits on the river Seine?".into(),
            correct_answer: "Paris".into(),
            incorrect_answers: vec!["Vienna".into(), "Prague".into(), "Rome".into()],
            difficulty: "easy".into(),
        },
        TriviaQuestion {
            category: "History".into(),
            question: "The Rosetta Stone was instrumental in deciphering which writing system?".into(),
            correct_answer: "Egyptian hieroglyphs".into(),
            incorrect_answers: vec!["Cuneiform".into(), "Linear B".into(), "Runic".into()],
            difficulty: "hard".into(),
        },
    ]
}

enum QbPhase {
    Reading,
    Buzzed,
    Correct,
    Incorrect,
    Timeout,
}

fn mock_tossup() -> Tossup {
    Tossup {
        question: "This author's debut novel follows a young man named Stephen Dedalus through \
                   Dublin. For 10 points, name this Irish modernist who also wrote a famously \
                   dense work set over a single day in June."
            .into(),
        answer: "James Joyce".into(),
        accepted_answers: vec!["james joyce".into(), "joyce".into()],
        category: "Literature".into(),
        difficulty: "College".into(),
    }
}

fn mock_qb(phase: QbPhase) -> TossupScreen {
    let mut s = TossupScreen::new(mock_tossup(), 45, 3);
    let total = s.tossup.question.chars().count();
    match phase {
        QbPhase::Reading => {
            s.chars_revealed = total / 3;
            s.char_timer = s.chars_revealed as f32 / 20.0;
            s.state = TossupState::Reading;
        }
        QbPhase::Buzzed => {
            s.chars_revealed = total / 2;
            s.state = TossupState::BuzzedIn { answer_input: String::new() };
        }
        QbPhase::Correct => {
            s.chars_revealed = total / 2;
            s.score += 10;
            s.state = TossupState::Result { correct: true, score_delta: 10 };
        }
        QbPhase::Incorrect => {
            s.chars_revealed = total / 2;
            s.score -= 5;
            s.state = TossupState::Result { correct: false, score_delta: -5 };
        }
        QbPhase::Timeout => {
            s.chars_revealed = total;
            s.state = TossupState::TimedOut;
        }
    }
    s
}

fn mock_bonus() -> Bonus {
    let part = |text: &str, ans: &str| BonusPart {
        text: text.into(),
        answer: ans.into(),
        accepted_answers: vec![ans.to_lowercase()],
    };
    Bonus {
        leadin: "This author wrote a cycle of novels collectively known as \u{201C}In Search of \
                 Lost Time.\u{201D} For 10 points each:"
            .into(),
        parts: vec![
            part("Name this French author of that modernist epic, famous for a madeleine.", "Marcel Proust"),
            part("In Search of Lost Time opens in this fictional town where the narrator spends his childhood summers.", "Combray"),
            part("Proust wrote in this language, also used by Flaubert and Balzac.", "French"),
        ],
        category: "Literature".into(),
        difficulty: "College".into(),
    }
}

fn mock_clue() -> Clue {
    Clue {
        category: "TRAVEL & TOURISM".into(),
        value: 600,
        question: "We're not stringing you along: this capital of the Czech Republic is famous \
                   for its puppet theatres."
            .into(),
        answer: "Prague".into(),
        round: "Jeopardy!".into(),
        air_date: "2019-03-14".into(),
    }
}

fn mock_board() -> Board {
    let cats = [
        "ROYAL NICKNAMES",
        "TV ACTORS & ROLES",
        "TRAVEL & TOURISM",
        "\u{201C}I\u{201D} LADS",
        "FOREWORDS",
        "BACKWORDS",
    ];
    let categories: Vec<(String, Vec<Clue>)> = cats
        .iter()
        .map(|name| {
            let clues = (0..5)
                .map(|r| Clue {
                    category: (*name).into(),
                    value: (r as i32 + 1) * 200,
                    question: format!("Sample clue for {name} worth ${}", (r + 1) * 200),
                    answer: "Sample answer".into(),
                    round: "Jeopardy!".into(),
                    air_date: "2019-03-14".into(),
                })
                .collect();
            ((*name).to_string(), clues)
        })
        .collect();
    Board::new(categories, false, vec![("YOU".into(), 1800)])
}
