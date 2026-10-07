use egui::{Align2, Pos2, Ui};
use crate::api::jeopardy::Clue;
use crate::api::opentdb::TriviaQuestion;
use crate::api::qbreader::Tossup;
use crate::screens::jeopardy::clue::{ClueAction, ClueScreen};
use crate::screens::kit::{self, SummaryAction};
use crate::screens::quizbowl::tossup::{TossupAction, TossupScreen};
use crate::screens::trivia::question::{TriviaAction, TriviaScreen};
use crate::theme;

pub const TRIVIA_COUNT: usize = 3;
pub const QB_COUNT: u32 = 3;
pub const JEOPARDY_COUNT: usize = 3;

#[derive(Debug, PartialEq)]
pub enum MixPhase {
    Loading,
    Trivia,
    QuizbowlLoading,
    Quizbowl,
    JeopardyPhase,
    Summary,
}

pub enum MixSignal {
    Quit,
    Restart,
    NeedQb { exclude_category: Option<String> },
}

pub struct MixScreen {
    pub phase: MixPhase,
    pub trivia: Option<TriviaScreen>,
    pub quizbowl: Option<TossupScreen>,
    pub quizbowl_done: u32,
    pub quizbowl_running_score: i32,
    jeopardy_clues: Vec<Clue>,
    jeopardy_idx: usize,
    jeopardy_clue: Option<ClueScreen>,
    pub trivia_score_final: i32,
    pub quizbowl_score_final: i32,
    pub jeopardy_score: i32,
    summary_start: f64,
}

impl MixScreen {
    pub fn new(jeopardy_clues: Vec<Clue>) -> Self {
        Self {
            phase: MixPhase::Loading,
            trivia: Some(TriviaScreen::loading()),
            quizbowl: None,
            quizbowl_done: 0,
            quizbowl_running_score: 0,
            jeopardy_clues,
            jeopardy_idx: 0,
            jeopardy_clue: None,
            trivia_score_final: 0,
            quizbowl_score_final: 0,
            jeopardy_score: 0,
            summary_start: 0.0,
        }
    }

    pub fn on_trivia_loaded(&mut self, result: Result<Vec<TriviaQuestion>, String>) {
        self.trivia = Some(match result {
            Ok(qs) => TriviaScreen::new(qs),
            Err(e) => TriviaScreen::with_error(e),
        });
        self.phase = MixPhase::Trivia;
    }

    pub fn on_qb_loaded(&mut self, result: Result<Tossup, String>) {
        let score = self.quizbowl_running_score;
        let done = self.quizbowl_done;
        self.quizbowl = Some(match result {
            Ok(t) => TossupScreen::new(t, score, done),
            Err(e) => {
                let mut s = TossupScreen::loading(score, done);
                s.loading = false;
                s.error = Some(e);
                s
            }
        });
        self.phase = MixPhase::Quizbowl;
    }

    pub fn show(&mut self, ui: &mut Ui, dt: f32) -> Option<MixSignal> {
        match self.phase {
            MixPhase::Loading => {
                kit::ambient(ui.ctx());
                kit::loader(ui, "Mixing up your questions…");
                None
            }

            MixPhase::Trivia => {
                let trivia = match self.trivia.as_mut() {
                    Some(t) => t,
                    None => return None,
                };

                // If all questions answered, transition before TriviaScreen renders its summary.
                if !trivia.loading
                    && !trivia.questions.is_empty()
                    && trivia.current_idx >= trivia.questions.len()
                {
                    self.trivia_score_final = trivia.score;
                    self.phase = MixPhase::QuizbowlLoading;
                    return Some(MixSignal::NeedQb { exclude_category: None });
                }

                match trivia.show(ui, dt) {
                    Some(TriviaAction::Quit) => Some(MixSignal::Quit),
                    Some(TriviaAction::Next { .. }) => {
                        let trivia = self.trivia.as_ref().unwrap();
                        if !trivia.questions.is_empty()
                            && trivia.current_idx >= trivia.questions.len()
                        {
                            self.trivia_score_final = trivia.score;
                            self.phase = MixPhase::QuizbowlLoading;
                            Some(MixSignal::NeedQb { exclude_category: None })
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }

            MixPhase::QuizbowlLoading => {
                kit::ambient(ui.ctx());
                kit::loader(ui, "Finding a tossup…");
                None
            }

            MixPhase::Quizbowl => {
                let qb = match self.quizbowl.as_mut() {
                    Some(q) => q,
                    None => return None,
                };

                match qb.show(ui, dt) {
                    Some(TossupAction::Quit) => Some(MixSignal::Quit),
                    Some(TossupAction::Next) | Some(TossupAction::Bonus) => {
                        let score = self.quizbowl.as_ref().map(|q| q.score).unwrap_or(0);
                        let cat = self
                            .quizbowl
                            .as_ref()
                            .map(|q| q.tossup.category.clone())
                            .unwrap_or_default();
                        self.quizbowl_done += 1;
                        if self.quizbowl_done >= QB_COUNT {
                            self.quizbowl_score_final = score;
                            self.phase = MixPhase::JeopardyPhase;
                            if !self.jeopardy_clues.is_empty() {
                                let mut cs = ClueScreen::new(self.jeopardy_clues[0].clone());
                                cs.next_label = "Next clue →";
                                self.jeopardy_clue = Some(cs);
                            }
                            None
                        } else {
                            self.quizbowl_running_score = score;
                            self.phase = MixPhase::QuizbowlLoading;
                            Some(MixSignal::NeedQb {
                                exclude_category: Some(cat),
                            })
                        }
                    }
                    None => None,
                }
            }

            MixPhase::JeopardyPhase => {
                kit::ambient(ui.ctx());

                // Progress bar and score overlay at top (same geometry as trivia bar)
                let avail_w = ui.available_width();
                let content_w = (avail_w - 88.0).min(900.0).max(420.0);
                let left = (avail_w - content_w) / 2.0;
                let right = left + content_w;
                let bar_cy = 30.0 + 15.0;
                let total_so_far =
                    self.trivia_score_final + self.quizbowl_score_final + self.jeopardy_score;

                ui.painter().text(
                    Pos2::new(left, bar_cy),
                    Align2::LEFT_CENTER,
                    format!(
                        "JEOPARDY   •   CLUE {}/{}",
                        self.jeopardy_idx + 1,
                        JEOPARDY_COUNT
                    ),
                    theme::mono(11.0),
                    theme::GOLD,
                );
                kit::score_chip(
                    ui,
                    Pos2::new(right, bar_cy),
                    "SCORE",
                    &total_so_far.to_string(),
                    0.0,
                );

                if kit::quit_button(ui) {
                    return Some(MixSignal::Quit);
                }

                // Init clue screen if needed
                if self.jeopardy_clue.is_none() {
                    if self.jeopardy_idx < self.jeopardy_clues.len() {
                        let mut cs = ClueScreen::new(
                            self.jeopardy_clues[self.jeopardy_idx].clone(),
                        );
                        cs.next_label = "Next clue →";
                        self.jeopardy_clue = Some(cs);
                    } else {
                        let now = ui.input(|i| i.time);
                        self.summary_start = now;
                        self.phase = MixPhase::Summary;
                        return None;
                    }
                }

                if let Some(clue_screen) = self.jeopardy_clue.as_mut() {
                    if let Some(ClueAction::Done { correct }) = clue_screen.show(ui) {
                        let value = self
                            .jeopardy_clues
                            .get(self.jeopardy_idx)
                            .map(|c| c.value)
                            .unwrap_or(200)
                            .max(200);
                        self.jeopardy_score += if correct { value } else { 0 };
                        self.jeopardy_idx += 1;
                        self.jeopardy_clue = None;

                        if self.jeopardy_idx >= JEOPARDY_COUNT {
                            let now = ui.input(|i| i.time);
                            self.summary_start = now;
                            self.phase = MixPhase::Summary;
                        }
                    }
                }

                None
            }

            MixPhase::Summary => {
                let total =
                    self.trivia_score_final + self.quizbowl_score_final + self.jeopardy_score;
                let stats = [
                    kit::Stat { value: self.trivia_score_final.to_string(), label: "trivia" },
                    kit::Stat {
                        value: self.quizbowl_score_final.to_string(),
                        label: "quizbowl",
                    },
                    kit::Stat { value: self.jeopardy_score.to_string(), label: "jeopardy" },
                ];
                match kit::summary(
                    ui,
                    "MIX COMPLETE",
                    &total.to_string(),
                    "TOTAL SCORE",
                    &stats,
                    self.summary_start,
                ) {
                    SummaryAction::Restart => Some(MixSignal::Restart),
                    SummaryAction::Quit => Some(MixSignal::Quit),
                    SummaryAction::None => None,
                }
            }
        }
    }
}
