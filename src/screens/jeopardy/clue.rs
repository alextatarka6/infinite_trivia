//! Jeopardy clue — "03 Clue" in the Roster hi-fi.

use egui::{Align2, Key, Pos2, Rect, Stroke, Vec2};
use crate::api::jeopardy::{check_answer, Clue};
use crate::screens::jeopardy::board::money;
use crate::screens::kit::{self, Btn, Canvas, RosterEntry, TagStyle, Texture};
use crate::theme::{self, GOOD, INK, MUTED, ON_BAD, ON_GOOD, BAD};

#[derive(Default)]
pub enum ClueState {
    #[default]
    ShowQuestion,
    Answered { correct: bool },
}

pub struct ClueScreen {
    pub clue: Clue,
    pub input: String,
    pub state: ClueState,
    pub next_label: &'static str,
    /// Teams + the one answering, copied from the board by App for the roster row.
    pub teams: Vec<(String, i32)>,
    pub active: usize,
    /// Format roster scores as dollars (false in Mix, which totals points).
    pub money_scores: bool,
    enter_start: f64,
    result_start: f64,
    focus_input: bool,
}

pub enum ClueAction {
    Done { correct: bool },
}

impl ClueScreen {
    pub fn new(clue: Clue) -> Self {
        Self {
            clue,
            input: String::new(),
            state: ClueState::default(),
            next_label: "Back to board",
            teams: Vec::new(),
            active: 0,
            money_scores: true,
            enter_start: -1.0,
            result_start: -10.0,
            focus_input: true,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<ClueAction> {
        kit::backdrop(ui.ctx(), Some(Texture::Grid));
        let c = Canvas::new(ui.ctx());
        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        let painter = ui.painter().clone();
        let mut action = None;

        // -- Tags: category + value ----------------------------------------------
        let cat = kit::tag(&painter, &c, c.p(80.0, 72.0), &self.clue.category.to_uppercase(), TagStyle::Panel, theme::TYPE_TAG);
        kit::tag(&painter, &c, Pos2::new(cat.right() - c.v(theme::SLANT_TAG), cat.top()), &money(self.clue.value), TagStyle::Gold, 20.0);

        // -- Clue -------------------------------------------------------------------------
        let q = self.clue.question.to_uppercase();
        let max_w = c.v(900.0);
        let size = kit::fit_size(&painter, &q, 600, c.v(theme::TYPE_CLUE), c.v(30.0), max_w, c.v(220.0), 1.15);
        kit::wrapped(&painter, c.p(80.0, 150.0), &q, theme::cond(600, size), INK, max_w, size * 1.15);

        let row_y = 430.0;
        match self.state {
            ClueState::ShowQuestion => {
                // Typed response (solo / host keyboard).
                kit::label(&painter, &c, c.p(80.0, row_y - 18.0), Align2::LEFT_CENTER, "YOUR RESPONSE", MUTED);
                let field = c.r(80.0, row_y, 480.0, 58.0);
                let submitted = kit::answer_field(ui, field, &mut self.input, self.focus_input, c.v(28.0));
                self.focus_input = false;
                let sb = c.r(568.0, row_y, 150.0, 58.0);
                if submitted || kit::button(ui, &c, sb, "SUBMIT", Btn::Primary, "clue_submit").clicked() {
                    self.answer(now);
                }
                kit::key_hint(&painter, &c, Pos2::new(sb.right() + c.v(12.0), sb.center().y), Align2::LEFT_CENTER, "ENTER ↵");

                // Or the host judges an answer given out loud.
                let jy = row_y + 96.0;
                let l = kit::label(&painter, &c, c.p(80.0, jy + 21.0), Align2::LEFT_CENTER, "ANSWER OUT LOUD · HOST JUDGES", MUTED);
                let x0 = (l.right() - c.origin.x) / c.s + 18.0;
                let keys = !ui.ctx().wants_keyboard_input();
                let ok = kit::button(ui, &c, c.r(x0, jy, 152.0, 42.0), "✓ CORRECT  C", Btn::Good, "clue_ok").clicked()
                    || (keys && ui.input(|i| i.key_pressed(Key::C)));
                let bad = kit::button(ui, &c, c.r(x0 + 168.0, jy, 138.0, 42.0), "✕ WRONG  X", Btn::Bad, "clue_bad").clicked()
                    || (keys && ui.input(|i| i.key_pressed(Key::X)));
                let reveal = kit::button(ui, &c, c.r(x0 + 322.0, jy, 110.0, 42.0), "REVEAL  R", Btn::Ghost, "clue_reveal").clicked()
                    || (keys && ui.input(|i| i.key_pressed(Key::R)));
                if ok || bad || reveal {
                    self.state = ClueState::Answered { correct: ok };
                    self.result_start = now;
                }
            }

            ClueState::Answered { correct } => {
                // Verdict band flips in (scaleY from the center line).
                let t = kit::ease_out(kit::since(ui, self.result_start, 0.3) / 0.3);
                let band = c.r(80.0, 390.0, 520.0, 76.0);
                let band = Rect::from_center_size(band.center(), Vec2::new(band.width(), band.height() * t.max(0.02)));
                let (fill, ink, word) = if correct { (GOOD, ON_GOOD, "CORRECT") } else { (BAD, ON_BAD, "WRONG") };
                kit::poly(&painter, kit::pattern_a(band, c.v(theme::CHAMFER_MD)), fill, Stroke::NONE);
                if t > 0.6 {
                    painter.text(Pos2::new(band.left() + c.v(24.0), band.center().y), Align2::LEFT_CENTER, word, c.cond(800, 52.0), ink);
                    let delta = if correct { format!("+{}", money(self.clue.value)) } else { format!("−{}", money(self.clue.value)) };
                    painter.text(Pos2::new(band.right() - c.v(24.0), band.center().y), Align2::RIGHT_CENTER, delta, c.cond(800, 36.0), ink);
                }
                kit::label(&painter, &c, c.p(80.0, 488.0), Align2::LEFT_CENTER, "CORRECT RESPONSE", MUTED);
                kit::wrapped(&painter, c.p(80.0, 500.0), &self.clue.answer.to_uppercase(), c.cond(800, 40.0), INK, c.v(700.0), c.v(42.0));
                let label = self.next_label.trim_end_matches(" →").to_uppercase();
                let size = kit::button_size(&painter, &c, &label, theme::TYPE_BUTTON, Vec2::new(24.0, 14.0));
                let br = Rect::from_min_size(c.p(80.0, 556.0), size);
                if kit::button(ui, &c, br, &label, Btn::Primary, "clue_next").clicked()
                    || ui.input(|i| i.key_pressed(Key::Enter))
                {
                    action = Some(ClueAction::Done { correct });
                }
                kit::key_hint(&painter, &c, Pos2::new(br.right() + c.v(12.0), br.center().y), Align2::LEFT_CENTER, "ENTER ↵");
            }
        }

        // -- Roster: the answering team is active; the rest dim. -----------------------
        if !self.teams.is_empty() {
            let note = if self.teams.len() > 1 { "ANSWERING" } else { "" };
            let entries: Vec<RosterEntry> = self
                .teams
                .iter()
                .enumerate()
                .map(|(i, (n, s))| {
                    let mut e = RosterEntry::new(n, *s);
                    e.money = self.money_scores;
                    if i == self.active && !note.is_empty() {
                        e.note = Some(note.into());
                    }
                    e
                })
                .collect();
            let w = (entries.len() as f32 * 400.0).min(1200.0);
            kit::roster(ui, &c, 40.0, w, 772.0, &entries, Some(self.active), entries.len() > 1);
        }

        action
    }

    pub fn answer(&mut self, now: f64) {
        let correct = check_answer(&self.input, &self.clue.answer);
        self.state = ClueState::Answered { correct };
        self.result_start = now;
    }

    /// Called by App when a phone player submits an answer.
    pub fn set_phone_answer(&mut self, text: &str, now: f64) {
        if matches!(self.state, ClueState::ShowQuestion) {
            self.input = text.to_string();
            self.answer(now);
        }
    }
}
