//! Final Jeopardy — the Daily Double layout with the category reveal first.
//!
//! Solo (no phones): the host types the wager and the response, as before.
//! Phone games: every team with a positive score wagers and answers on its
//! phone. The host advances with ENTER once everyone is in (or to move on
//! without stragglers: a missing wager counts as $0, a missing answer as wrong).

use egui::{Align2, Key, Pos2, Rect, Vec2};
use crate::api::jeopardy::{check_answer, Clue};
use crate::screens::jeopardy::wager;
use crate::screens::kit::{self, Btn, Canvas, RosterEntry, Texture};
use crate::theme::{self, BAD, GOLD, GOOD, INK, MUTED};

pub enum FinalAction {
    /// Score change for every team, in `teams` order.
    Done { deltas: Vec<i32> },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FinalPhase {
    Wager,
    Clue,
    Result,
}

pub struct FinalJeopardy {
    pub clue: Clue,
    pub phase: FinalPhase,
    /// Teams and their pre-Final scores, copied from the board by App.
    pub teams: Vec<(String, i32)>,
    /// True when teams play on phones; false = the host types for one player.
    pub phones: bool,
    pub wagers: Vec<Option<i32>>,
    /// Each team's response and whether it was judged correct.
    pub responses: Vec<Option<(String, bool)>>,
    pub wager_input: String,
    pub answer_input: String,
    enter_start: f64,
    result_start: f64,
    focus_field: bool,
}

impl FinalJeopardy {
    pub fn new(clue: Clue, teams: Vec<(String, i32)>, phones: bool) -> Self {
        let n = teams.len().max(1);
        let mut f = Self {
            clue,
            phase: FinalPhase::Wager,
            teams,
            phones,
            wagers: vec![None; n],
            responses: vec![None; n],
            wager_input: String::new(),
            answer_input: String::new(),
            enter_start: -1.0,
            result_start: -10.0,
            focus_field: true,
        };
        // Teams at or below $0 sit Final out.
        for i in 0..n {
            if !f.can_play(i) {
                f.wagers[i] = Some(0);
            }
        }
        f
    }

    fn score(&self, i: usize) -> i32 {
        self.teams.get(i).map(|t| t.1).unwrap_or(0)
    }

    /// Only teams with money play Final (a solo player always plays).
    pub fn can_play(&self, i: usize) -> bool {
        !self.phones || self.score(i) > 0
    }

    pub fn max_wager(&self, i: usize) -> i32 {
        self.score(i).max(0)
    }

    pub fn delta(&self, i: usize) -> i32 {
        let w = self.wagers.get(i).copied().flatten().unwrap_or(0);
        match self.responses.get(i).cloned().flatten() {
            Some((_, true)) => w,
            _ => -w,
        }
    }

    /// A phone's wager, accepted once per team during the wager phase.
    pub fn phone_wager(&mut self, i: usize, amount: i32) {
        if self.phase == FinalPhase::Wager && self.can_play(i) && self.wagers.get(i) == Some(&None) {
            self.wagers[i] = Some(amount.clamp(0, self.max_wager(i)));
        }
    }

    /// A phone's response, accepted once per team during the clue phase.
    pub fn phone_answer(&mut self, i: usize, text: &str) {
        if self.phase == FinalPhase::Clue && self.can_play(i) && self.responses.get(i) == Some(&None) {
            let correct = check_answer(text, &self.clue.answer);
            self.responses[i] = Some((text.to_string(), correct));
        }
    }

    fn players(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.wagers.len()).filter(|&i| self.can_play(i))
    }

    fn all_wagered(&self) -> bool {
        self.players().all(|i| self.wagers[i].is_some())
    }

    fn all_answered(&self) -> bool {
        self.players().all(|i| self.responses[i].is_some())
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<FinalAction> {
        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        if self.phones { self.show_phones(ui, now) } else { self.show_solo(ui, now) }
    }

    fn show_solo(&mut self, ui: &mut egui::Ui, now: f64) -> Option<FinalAction> {
        let c = Canvas::new(ui.ctx());
        let enter = kit::since(ui, self.enter_start, 0.5);
        let team = self.teams.first().map(|(n, _)| n.clone()).unwrap_or_else(|| "You".into());
        let score = self.score(0);
        let mut action = None;
        match self.phase {
            FinalPhase::Wager => {
                kit::backdrop(ui.ctx(), None);
                wager::band(ui, &c, "FINAL JEOPARDY", 1.0, enter);
                category_reveal(ui, &c, &self.clue.category);
                let max = self.max_wager(0);
                if let Some(w) = wager::wager_panel(ui, &c, "YOUR WAGER", &mut self.wager_input, 0, max, self.focus_field, "fj") {
                    self.wagers[0] = Some(w);
                    self.phase = FinalPhase::Clue;
                    self.focus_field = true;
                } else {
                    self.focus_field = false;
                }
                wager::team_line(ui, &c, &team, score, "BET IT ALL OR PLAY IT SAFE");
            }
            FinalPhase::Clue => {
                kit::backdrop(ui.ctx(), Some(Texture::Grid));
                let title = format!("FINAL JEOPARDY · {}", kit::money(self.wagers[0].unwrap_or(0)));
                wager::band(ui, &c, &title, 0.0, 1.0);
                if wager::question(ui, &c, &self.clue.category, &self.clue.question, &mut self.answer_input, self.focus_field, "fj") {
                    let correct = check_answer(&self.answer_input, &self.clue.answer);
                    self.responses[0] = Some((self.answer_input.clone(), correct));
                    self.phase = FinalPhase::Result;
                    self.result_start = now;
                }
                self.focus_field = false;
            }
            FinalPhase::Result => {
                kit::backdrop(ui.ctx(), Some(Texture::Grid));
                wager::band(ui, &c, "FINAL JEOPARDY", 0.0, 1.0);
                let correct = matches!(self.responses[0], Some((_, true)));
                let delta = self.delta(0);
                let extra = format!("FINAL SCORE {}", kit::money(score + delta));
                let since = kit::since(ui, self.result_start, 0.4);
                if wager::result(ui, &c, since, correct, delta, &self.clue.answer, Some(&extra), "Final standings", "fj") {
                    action = Some(FinalAction::Done { deltas: vec![delta] });
                }
            }
        }
        action
    }

    fn show_phones(&mut self, ui: &mut egui::Ui, now: f64) -> Option<FinalAction> {
        let c = Canvas::new(ui.ctx());
        let enter = kit::since(ui, self.enter_start, 0.5);
        let enter_pressed = ui.input(|i| i.key_pressed(Key::Enter));
        let painter = ui.painter().clone();
        let mut action = None;

        match self.phase {
            FinalPhase::Wager => {
                kit::backdrop(ui.ctx(), None);
                wager::band(ui, &c, "FINAL JEOPARDY", 1.0, enter);
                category_reveal(ui, &c, &self.clue.category);
                kit::label(&painter, &c, c.p(120.0, 486.0), Align2::LEFT_CENTER, "WAGER ON YOUR PHONES", MUTED);
                let done = self.players().filter(|&i| self.wagers[i].is_some()).count();
                let total = self.players().count();
                painter.text(c.p(120.0, 530.0), Align2::LEFT_CENTER, format!("{done} OF {total} LOCKED"),
                    c.cond(800, 44.0), if done == total { GOLD } else { INK });
                let label = if self.all_wagered() { "REVEAL THE CLUE" } else { "REVEAL ANYWAY" };
                if advance_button(ui, &c, &painter, label, 120.0, 574.0, "fj_reveal") || enter_pressed {
                    for w in self.wagers.iter_mut() {
                        w.get_or_insert(0);
                    }
                    self.phase = FinalPhase::Clue;
                }
                self.roster(ui, &c, |s, i| match s.wagers[i] {
                    _ if !s.can_play(i) => "SITTING OUT",
                    Some(_) => "LOCKED",
                    None => "WAGERING",
                });
            }
            FinalPhase::Clue => {
                kit::backdrop(ui.ctx(), Some(Texture::Grid));
                wager::band(ui, &c, "FINAL JEOPARDY", 0.0, 1.0);
                kit::label(&painter, &c, c.p(80.0, 140.0), Align2::LEFT_CENTER, &self.clue.category.to_uppercase(), GOLD);
                let q = self.clue.question.to_uppercase();
                let max_w = c.v(1000.0);
                let size = kit::fit_size(&painter, &q, 600, c.v(theme::TYPE_CLUE), c.v(30.0), max_w, c.v(240.0), 1.15);
                kit::wrapped(&painter, c.p(80.0, 162.0), &q, theme::cond(600, size), INK, max_w, size * 1.15);
                let done = self.players().filter(|&i| self.responses[i].is_some()).count();
                let total = self.players().count();
                kit::label(&painter, &c, c.p(80.0, 452.0), Align2::LEFT_CENTER, "ANSWER ON YOUR PHONES", MUTED);
                painter.text(c.p(80.0, 494.0), Align2::LEFT_CENTER, format!("{done} OF {total} IN"),
                    c.cond(800, 44.0), if done == total { GOLD } else { INK });
                let label = if self.all_answered() { "REVEAL ANSWERS" } else { "STOP ANSWERING" };
                if advance_button(ui, &c, &painter, label, 80.0, 534.0, "fj_answers") || enter_pressed {
                    self.phase = FinalPhase::Result;
                    self.result_start = now;
                }
                self.roster(ui, &c, |s, i| match s.responses[i] {
                    _ if !s.can_play(i) => "SITTING OUT",
                    Some(_) => "ANSWERED",
                    None => "WRITING",
                });
            }
            FinalPhase::Result => {
                kit::backdrop(ui.ctx(), Some(Texture::Grid));
                wager::band(ui, &c, "FINAL JEOPARDY", 0.0, 1.0);
                kit::label(&painter, &c, c.p(80.0, 140.0), Align2::LEFT_CENTER, "CORRECT RESPONSE", MUTED);
                let r = kit::wrapped(&painter, c.p(80.0, 156.0), &self.clue.answer.to_uppercase(), c.cond(800, 48.0), INK, c.v(1000.0), c.v(50.0));
                let mut y = (r.bottom() - c.origin.y) / c.s + 24.0;
                let t = kit::ease_out(kit::since(ui, self.result_start, 0.4) / 0.4);
                for i in self.players().collect::<Vec<_>>() {
                    if y > 560.0 {
                        break;
                    }
                    let name = self.teams[i].0.to_uppercase();
                    let (said, ok) = self.responses[i].clone().unwrap_or_else(|| ("NO ANSWER".into(), false));
                    let col = if ok { GOOD } else { BAD };
                    painter.text(c.p(80.0, y + 16.0), Align2::LEFT_CENTER, &name, c.cond(800, 26.0), INK.gamma_multiply(t));
                    painter.text(c.p(360.0, y + 16.0), Align2::LEFT_CENTER, format!("“{}”", said.to_uppercase()),
                        c.cond(600, 24.0), MUTED.gamma_multiply(t));
                    let d = self.delta(i);
                    let txt = if d >= 0 { format!("+{}", kit::money(d)) } else { kit::money(d) };
                    painter.text(c.p(1160.0, y + 16.0), Align2::RIGHT_CENTER, txt, c.cond(800, 28.0), col.gamma_multiply(t));
                    y += 40.0;
                }
                if advance_button(ui, &c, &painter, "FINAL STANDINGS", 80.0, y + 16.0, "fj_done") || enter_pressed {
                    let deltas = (0..self.teams.len()).map(|i| self.delta(i)).collect();
                    action = Some(FinalAction::Done { deltas });
                }
                let entries: Vec<RosterEntry> = self.teams.iter().enumerate().map(|(i, (n, s))| {
                    let d = self.delta(i);
                    let mut e = RosterEntry::new(n, s + d);
                    e.money = true;
                    if self.can_play(i) {
                        e.delta = Some(d);
                        e.top = Some(if matches!(self.responses[i], Some((_, true))) { GOOD } else { BAD });
                    }
                    e
                }).collect();
                kit::roster(ui, &c, 40.0, 1200.0, 772.0, &entries, None, false);
            }
        }
        action
    }

    fn roster(&self, ui: &egui::Ui, c: &Canvas, status: impl Fn(&Self, usize) -> &'static str) {
        let entries: Vec<RosterEntry> = self.teams.iter().enumerate().map(|(i, (n, s))| {
            let mut e = RosterEntry::new(n, *s);
            e.money = true;
            e.status = Some(status(self, i));
            e
        }).collect();
        kit::roster(ui, c, 40.0, 1200.0, 772.0, &entries, None, false);
    }
}

fn category_reveal(ui: &egui::Ui, c: &Canvas, category: &str) {
    let p = ui.painter();
    kit::label(p, c, c.p(700.0, 486.0), Align2::LEFT_CENTER, "THE CATEGORY", MUTED);
    kit::wrapped(p, c.p(698.0, 500.0), &category.to_uppercase(), c.cond(800, 40.0), INK, c.v(460.0), c.v(40.0));
}

#[allow(clippy::too_many_arguments)]
fn advance_button(ui: &mut egui::Ui, c: &Canvas, painter: &egui::Painter, label: &str, x: f32, y: f32, id: &str) -> bool {
    let size = kit::button_size(painter, c, label, theme::TYPE_BUTTON, Vec2::new(24.0, 14.0));
    let br = Rect::from_min_size(c.p(x, y), size);
    let go = kit::button(ui, c, br, label, Btn::Primary, id).clicked();
    kit::key_hint(painter, c, Pos2::new(br.right() + c.v(12.0), br.center().y), Align2::LEFT_CENTER, "ENTER ↵");
    go
}
