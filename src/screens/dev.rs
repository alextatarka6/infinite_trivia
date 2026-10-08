//! Dev-only screen gallery. Jumps straight into any in-game screen with mock
//! data so the UI can be evaluated without the API or playing through.
//! Enabled by launching with the env var `IT_DEV=1`.

use egui::{Align2, Sense};

use crate::api::jeopardy::Clue;
use crate::api::opentdb::TriviaQuestion;
use crate::api::qbreader::{Bonus, BonusPart, Tossup};
use crate::screens::quizbowl::bonus::BonusScreen;
use crate::screens::jeopardy::board::{Board, BoardAction};
use crate::screens::jeopardy::clue::{ClueScreen, ClueState};
use crate::screens::jeopardy::daily_double::DailyDoubleScreen;
use crate::screens::jeopardy::final_jeopardy::{FinalJeopardy, FinalPhase};
use crate::screens::jeopardy::setup::{JeopardySetupScreen, SetupAction};
use crate::screens::mix::{MixPhase, MixScreen, MixSignal};
use crate::screens::kit;
use crate::screens::lobby::{LobbyAction, LobbyScreen};
use crate::server::protocol::PlayerInfo;
use crate::screens::quizbowl::tossup::{TossupAction, TossupScreen, TossupState};
use crate::screens::trivia::question::{TriviaAction, TriviaScreen};
use crate::theme::{GOLD, MUTED};

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
    Mix(MixScreen),
    Lobby(LobbyScreen),
    Standings,
}

pub struct DevMenu {
    pub view: Option<DevView>,
}

impl Default for DevMenu {
    fn default() -> Self {
        // `IT_SCREEN=<key>` opens a screen directly (skips the gallery click).
        let view = std::env::var("IT_SCREEN").ok().and_then(|k| match k.as_str() {
            "trivia" => Some(DevView::Trivia(mock_trivia())),
            "trivia-reveal" => {
                let mut t = mock_trivia();
                t.current_idx = 1;
                t.score = 140;
                t.streak = 2;
                t.selected = Some(2);
                Some(DevView::Trivia(t))
            }
            "trivia-summary" => Some(DevView::Trivia(mock_trivia_summary())),
            "trivia-loading" => Some(DevView::Trivia(TriviaScreen::loading())),
            "qb" | "qb-reading" => Some(DevView::Quizbowl(mock_qb(QbPhase::Reading))),
            "qb-buzzed" => Some(DevView::Quizbowl(mock_qb(QbPhase::Buzzed))),
            "qb-correct" => Some(DevView::Quizbowl(mock_qb(QbPhase::Correct))),
            "qb-incorrect" => Some(DevView::Quizbowl(mock_qb(QbPhase::Incorrect))),
            "qb-timeout" => Some(DevView::Quizbowl(mock_qb(QbPhase::Timeout))),
            "bonus" => Some(DevView::Bonus(BonusScreen::new(mock_bonus(), 55, 3))),
            "setup" => Some(DevView::Setup(JeopardySetupScreen::default())),
            "setup-teams" => Some(DevView::Setup(JeopardySetupScreen::default())),
            "board" => Some(DevView::Board(mock_board())),
            "clue" => Some(DevView::Clue(mock_clue_screen())),
            "clue-result" => {
                let mut c = mock_clue_screen();
                c.state = ClueState::Answered { correct: true };
                Some(DevView::Clue(c))
            }
            "dd" => Some(DevView::DailyDouble(mock_dd())),
            "dd-clue" => {
                let mut d = mock_dd();
                d.wager = Some(1500);
                Some(DevView::DailyDouble(d))
            }
            "dd-result" => {
                let mut d = mock_dd();
                d.wager = Some(1200);
                d.answered = Some(false);
                Some(DevView::DailyDouble(d))
            }
            "final" => Some(DevView::Final(mock_final())),
            "final-result" => {
                let mut f = mock_final();
                f.phase = FinalPhase::Result;
                f.wagers[0] = Some(2000);
                f.responses[0] = Some(("Prague".into(), true));
                Some(DevView::Final(f))
            }
            "final-teams" => Some(DevView::Final(mock_final_teams(FinalPhase::Wager))),
            "final-teams-clue" => Some(DevView::Final(mock_final_teams(FinalPhase::Clue))),
            "final-teams-result" => Some(DevView::Final(mock_final_teams(FinalPhase::Result))),
            "lobby" => Some(DevView::Lobby(LobbyScreen::new("http://192.168.1.4:7777"))),
            "standings" => Some(DevView::Standings),
            "mix" => Some(DevView::Mix(mock_mix(MixPhase::JeopardyPhase))),
            "mix-summary" => Some(DevView::Mix(mock_mix(MixPhase::Summary))),
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
                DevView::Mix(s) => matches!(
                    s.show(ui, dt),
                    Some(MixSignal::Quit) | Some(MixSignal::Restart)
                ),
                DevView::Lobby(s) => matches!(s.show(ui, &mock_players()), Some(LobbyAction::Back)),
                DevView::Standings => {
                    let standings: Vec<kit::Standing> = [("Owls", 7200), ("Moths", 5000), ("Eels", -600)]
                        .iter()
                        .map(|(n, v)| kit::Standing { name: (*n).into(), score: kit::money(*v), negative: *v < 0 })
                        .collect();
                    let stats = [
                        kit::Stat { value: "24/30".into(), label: "clues answered" },
                        kit::Stat { value: "$3,000".into(), label: "biggest dd win" },
                        kit::Stat { value: "Cheese".into(), label: "untouched category" },
                    ];
                    !matches!(kit::round_complete(ui, "Jeopardy!", &standings, &stats, 0.0), kit::SummaryAction::None)
                }
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
        kit::backdrop(ui.ctx(), None);
        let c = kit::Canvas::new(ui.ctx());
        let p = ui.painter().clone();
        let mut go_home = false;
        kit::tag(&p, &c, c.p(64.0, 40.0), "DEV", kit::TagStyle::Gold, 16.0);
        p.text(c.p(64.0, 92.0), Align2::LEFT_TOP, "SCREEN GALLERY", c.cond(800, 46.0), GOLD);
        kit::label(&p, &c, c.p(64.0, 160.0), Align2::LEFT_CENTER, "JUMP INTO ANY SCREEN WITH MOCK DATA", MUTED);

        let buttons: &[(&str, fn() -> DevView)] = &[
            ("TRIVIA · QUESTION", || DevView::Trivia(mock_trivia())),
            ("TRIVIA · SUMMARY", || DevView::Trivia(mock_trivia_summary())),
            ("TRIVIA · LOADING", || DevView::Trivia(TriviaScreen::loading())),
            ("QUIZBOWL · READING", || DevView::Quizbowl(mock_qb(QbPhase::Reading))),
            ("QUIZBOWL · BUZZED", || DevView::Quizbowl(mock_qb(QbPhase::Buzzed))),
            ("QUIZBOWL · CORRECT", || DevView::Quizbowl(mock_qb(QbPhase::Correct))),
            ("QUIZBOWL · INCORRECT", || DevView::Quizbowl(mock_qb(QbPhase::Incorrect))),
            ("QUIZBOWL · TIMEOUT", || DevView::Quizbowl(mock_qb(QbPhase::Timeout))),
            ("QUIZBOWL · LOADING", || DevView::Quizbowl(TossupScreen::loading(0, 0))),
            ("QUIZBOWL · BONUS", || DevView::Bonus(BonusScreen::new(mock_bonus(), 55, 3))),
            ("JEOPARDY · SETUP", || DevView::Setup(JeopardySetupScreen::default())),
            ("JEOPARDY · BOARD", || DevView::Board(mock_board())),
            ("JEOPARDY · CLUE", || DevView::Clue(mock_clue_screen())),
            ("JEOPARDY · DAILY DOUBLE", || DevView::DailyDouble(mock_dd())),
            ("JEOPARDY · FINAL", || DevView::Final(mock_final())),
            ("JEOPARDY · FINAL (PHONES)", || DevView::Final(mock_final_teams(FinalPhase::Wager))),
            ("MIX · JEOPARDY PHASE", || DevView::Mix(mock_mix(MixPhase::JeopardyPhase))),
            ("MIX · SUMMARY", || DevView::Mix(mock_mix(MixPhase::Summary))),
            ("LOBBY", || DevView::Lobby(LobbyScreen::new("http://192.168.1.4:7777"))),
            ("ROUND COMPLETE · TEAMS", || DevView::Standings),
        ];
        for (i, (label, make)) in buttons.iter().enumerate() {
            let (col, row) = (i % 4, i / 4);
            let r = c.r(64.0 + col as f32 * 290.0, 200.0 + row as f32 * 62.0, 280.0, 52.0);
            if kit::button(ui, &c, r, label, kit::Btn::Secondary, ("dev", i)).clicked() {
                self.view = Some(make());
            }
        }
        if kit::button(ui, &c, c.r(64.0, 540.0, 260.0, 56.0), "REAL HOME SCREEN", kit::Btn::Primary, "dev_home").clicked() {
            go_home = true;
        }
        go_home
    }
}

/// Small "GALLERY" chip at the top center (clear of tags and QUIT).
fn dev_back_button(ui: &mut egui::Ui) -> bool {
    let c = kit::Canvas::new(ui.ctx());
    let rect = c.r(590.0, 4.0, 100.0, 22.0);
    let resp = ui.interact(rect, egui::Id::new("dev_back"), Sense::click());
    let t = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    ui.painter().rect_filled(rect, 0.0, kit::lerp_color(crate::theme::PANEL, crate::theme::PANEL_HI, t));
    kit::text_tracked(ui.painter(), rect.center(), Align2::CENTER_CENTER, "← GALLERY", c.cond(700, 11.0), MUTED, 0.14);
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
    s.score = 50;
    s.correct_count = 2;
    s.best_streak = 2;
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
            shuffle_seed: 1,
        },
        TriviaQuestion {
            category: "Geography".into(),
            question: "Which capital city sits on the river Seine?".into(),
            correct_answer: "Paris".into(),
            incorrect_answers: vec!["Vienna".into(), "Prague".into(), "Rome".into()],
            difficulty: "easy".into(),
            shuffle_seed: 2,
        },
        TriviaQuestion {
            category: "History".into(),
            question: "The Rosetta Stone was instrumental in deciphering which writing system?".into(),
            correct_answer: "Egyptian hieroglyphs".into(),
            incorrect_answers: vec!["Cuneiform".into(), "Linear B".into(), "Runic".into()],
            difficulty: "hard".into(),
            shuffle_seed: 3,
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
    // Three teams with a few spent tiles, matching the Roster hi-fi.
    let mut b = Board::new(
        categories,
        false,
        vec![("Owls".into(), 2400), ("Moths".into(), 1800), ("Eels".into(), -400)],
    );
    for (c, r) in [(0, 0), (1, 1), (1, 2), (1, 3), (2, 0), (2, 1), (2, 4), (4, 0)] {
        b.used[c][r] = true;
    }
    b.dd_left = 1;
    b
}

fn mock_final() -> FinalJeopardy {
    FinalJeopardy::new(mock_clue(), vec![("You".into(), 5200)], false)
}

/// Phone Final with three teams: Eels sit out (negative score), Owls are in.
fn mock_final_teams(phase: FinalPhase) -> FinalJeopardy {
    let mut f = FinalJeopardy::new(mock_clue(), mock_teams(), true);
    f.phone_wager(0, 2000);
    if phase != FinalPhase::Wager {
        f.phone_wager(1, 1800);
        f.phase = FinalPhase::Clue;
        f.phone_answer(0, "prague");
    }
    if phase == FinalPhase::Result {
        f.phone_answer(1, "Vienna");
        f.phase = FinalPhase::Result;
    }
    f
}

fn mock_players() -> Vec<PlayerInfo> {
    ["Owls", "Moths", "Eels"]
        .iter()
        .enumerate()
        .map(|(i, n)| PlayerInfo { id: i as u32 + 1, name: (*n).into(), score: 0, connected: i != 2 })
        .collect()
}

fn mock_teams() -> Vec<(String, i32)> {
    vec![("Owls".into(), 2400), ("Moths".into(), 1800), ("Eels".into(), -400)]
}

fn mock_clue_screen() -> ClueScreen {
    let mut c = ClueScreen::new(mock_clue());
    c.teams = mock_teams();
    c.active = 1;
    c
}

fn mock_dd() -> DailyDoubleScreen {
    let mut d = DailyDoubleScreen::new(mock_clue(), 2400);
    d.teams = mock_teams();
    d.wager_input = "1500".into();
    d
}

fn mock_mix(phase: MixPhase) -> MixScreen {
    let clues = vec![mock_clue(), mock_clue(), mock_clue()];
    let mut m = MixScreen::new(clues);
    m.trivia_score_final = 80;
    m.quizbowl_score_final = 45;
    match phase {
        MixPhase::JeopardyPhase => {
            m.phase = MixPhase::JeopardyPhase;
        }
        MixPhase::Summary => {
            m.phase = MixPhase::Summary;
            m.jeopardy_score = 600;
        }
        _ => {}
    }
    m
}
