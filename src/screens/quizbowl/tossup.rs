//! Quizbowl tossup — "06 Quizbowl" in the Roster hi-fi.

use egui::{Align2, Id, Key, Pos2, Rect, Sense, Stroke, TextFormat, Vec2};
use egui::text::LayoutJob;
use crate::api::qbreader::{check_answer, Tossup};
use crate::game::scoring::quizbowl_score;
use crate::screens::kit::{self, Btn, Canvas, RosterEntry, TagStyle, Texture};
use crate::screens::trivia::question::error_screen;
use crate::theme::{self, BAD, GOLD, GOOD, INK, MUTED, ON_BAD, ON_GOLD, ON_GOOD, PANEL};

const CHARS_PER_SECOND: f32 = 20.0;

pub enum TossupAction {
    Next,
    /// Correct tossup — advance to a bonus round before the next tossup.
    Bonus,
    Quit,
}

pub enum TossupState {
    Reading,
    FinishedReading { elapsed: f32 },
    BuzzedIn { answer_input: String },
    Result { correct: bool, score_delta: i32 },
    TimedOut,
}

pub struct TossupScreen {
    pub tossup: Tossup,
    pub score: i32,
    pub questions_done: u32,
    pub chars_revealed: usize,
    pub char_timer: f32,
    pub state: TossupState,
    pub loading: bool,
    pub error: Option<String>,
    /// Set when a phone player has buzzed; shows their name instead of the host answer input.
    pub mp_buzzed_name: Option<String>,
    /// Phone players' (name, score), set by App during a phone game.
    pub phones: Vec<(String, i32)>,
    // animation bookkeeping
    enter_start: f64,
    result_start: f64,
    bump_start: f64,
    focus_answer: bool,
}

impl TossupScreen {
    pub fn new(tossup: Tossup, score: i32, questions_done: u32) -> Self {
        Self {
            tossup,
            score,
            questions_done,
            chars_revealed: 0,
            char_timer: 0.0,
            state: TossupState::Reading,
            loading: false,
            error: None,
            mp_buzzed_name: None,
            phones: Vec::new(),
            enter_start: -1.0,
            result_start: -10.0,
            bump_start: -10.0,
            focus_answer: false,
        }
    }

    pub fn loading(score: i32, questions_done: u32) -> Self {
        let dummy = Tossup {
            question: String::new(),
            answer: String::new(),
            accepted_answers: vec![],
            category: String::new(),
            difficulty: String::new(),
        };
        let mut s = Self::new(dummy, score, questions_done);
        s.loading = true;
        s
    }

    /// Called by App when a phone player answers after buzzing.
    /// Returns (correct, score_delta) and transitions to the Result state.
    pub fn phone_answer(&mut self, text: &str, now: f64) -> (bool, i32) {
        let total = self.tossup.question.chars().count();
        let correct = check_answer(text, &self.tossup.accepted_answers);
        let delta = quizbowl_score(correct, self.chars_revealed, total);
        self.score += delta;
        self.chars_revealed = total;
        self.state = TossupState::Result { correct, score_delta: delta };
        self.result_start = now;
        self.bump_start = now;
        self.mp_buzzed_name = None;
        (correct, delta)
    }

    pub fn show(&mut self, ui: &mut egui::Ui, dt: f32) -> Option<TossupAction> {
        kit::backdrop(ui.ctx(), Some(Texture::Scan));
        if self.loading {
            kit::loading(ui, "FINDING A TOSSUP", "QUIZBOWL · QBREADER", None);
            return None;
        }
        if let Some(err) = self.error.clone() {
            return error_screen(ui, "COULD NOT FETCH A TOSSUP", &err).then_some(TossupAction::Quit);
        }

        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        if kit::quit_button(ui) {
            return Some(TossupAction::Quit);
        }
        let total_chars = self.tossup.question.chars().count();

        // -- Advance the reading ---------------------------------------------------------
        if matches!(self.state, TossupState::Reading) {
            self.char_timer += dt;
            self.chars_revealed = ((self.char_timer * CHARS_PER_SECOND) as usize).min(total_chars);
            if self.chars_revealed >= total_chars {
                self.state = TossupState::FinishedReading { elapsed: 0.0 };
            }
            ui.ctx().request_repaint();
        }
        if let TossupState::FinishedReading { ref mut elapsed } = self.state {
            *elapsed += dt;
            ui.ctx().request_repaint();
            if *elapsed >= 5.0 {
                self.state = TossupState::TimedOut;
            }
        }

        let c = Canvas::new(ui.ctx());
        let painter = ui.painter().clone();
        let revealed: String = self.tossup.question.chars().take(self.chars_revealed).collect();
        let remaining: String = self.tossup.question.chars().skip(self.chars_revealed).collect();

        // -- Top bar: TOSSUP tag, category, points ----------------------------------------
        let tag = kit::tag(&painter, &c, c.p(72.0, 44.0), &format!("TOSSUP {}", self.questions_done + 1), TagStyle::Gold, theme::TYPE_TAG);
        let mut meta = self.tossup.category.to_uppercase();
        if !self.tossup.difficulty.is_empty() {
            meta = format!("{meta} · {}", self.tossup.difficulty.to_uppercase());
        }
        kit::label(&painter, &c, Pos2::new(tag.right() + c.v(16.0), tag.center().y), Align2::LEFT_CENTER, &meta, MUTED);
        // Points: what a buzz is worth (the real scoring, see game/scoring.rs).
        let mut x = 1130.0;
        for (text, fill, ink) in [("−5 EARLY MISS", PANEL, BAD), ("+10 CORRECT", GOLD, ON_GOLD)] {
            let w = kit::text_width(&painter, text, c.cond(800, 15.0), 0.06) / c.s + 26.0;
            x -= w;
            let r = c.r(x, 44.0, w, 36.0);
            painter.rect_filled(r, 0.0, fill);
            kit::text_tracked(&painter, r.center(), Align2::CENTER_CENTER, text, c.cond(800, 15.0), ink, 0.06);
            x -= 4.0;
        }

        // -- Tossup text: read part in ink, a gold cursor bar; the rest only after the result.
        let max_w = c.v(990.0);
        let show_rest = matches!(self.state, TossupState::Result { .. } | TossupState::TimedOut);
        let full_len = if show_rest { total_chars } else { self.chars_revealed };
        let mut size = c.v(theme::TYPE_QB_TEXT);
        let measure = |sz: f32| {
            let mut job = LayoutJob::default();
            job.wrap.max_width = max_w;
            let all: String = self.tossup.question.chars().take(full_len.max(1)).collect();
            job.append(&all, 0.0, TextFormat { font_id: theme::barlow(500, sz), line_height: Some(sz * 1.38), ..Default::default() });
            painter.layout_job(job).size().y
        };
        // Shrink long tossups so the whole question fits above the buzzer.
        let full_h = {
            let mut job = LayoutJob::default();
            job.wrap.max_width = max_w;
            job.append(&self.tossup.question, 0.0, TextFormat { font_id: theme::barlow(500, size), line_height: Some(size * 1.38), ..Default::default() });
            painter.layout_job(job).size().y
        };
        if full_h > c.v(360.0) {
            size *= (c.v(360.0) / full_h).sqrt().max(0.6);
            while size > c.v(18.0) && measure(size) > c.v(360.0) {
                size -= 1.0;
            }
        }
        let mut job = LayoutJob::default();
        job.wrap.max_width = max_w;
        job.append(&revealed, 0.0, TextFormat { font_id: theme::barlow(500, size), color: INK, line_height: Some(size * 1.38), ..Default::default() });
        if show_rest && !remaining.is_empty() {
            job.append(&remaining, 0.0, TextFormat { font_id: theme::barlow(500, size), color: theme::alpha(MUTED, 0.45), line_height: Some(size * 1.38), ..Default::default() });
        }
        let galley = painter.layout_job(job);
        let top = c.p(72.0, 130.0);
        painter.galley(top, galley.clone(), INK);
        if matches!(self.state, TossupState::Reading | TossupState::FinishedReading { .. } | TossupState::BuzzedIn { .. }) {
            if let Some(row) = galley.rows.last() {
                let x = top.x + row.rect.right() + c.v(4.0);
                let cy = top.y + row.rect.center().y;
                painter.rect_filled(Rect::from_center_size(Pos2::new(x, cy), Vec2::new(c.v(4.0), size * 1.05)), 0.0, GOLD);
            }
        }

        // -- Footer per phase -----------------------------------------------------------------
        let mut buzz = false;
        let mut submitted: Option<String> = None;
        let mut next = false;
        let mut bonus = false;
        let words_left = remaining.split_whitespace().count();

        match self.state {
            TossupState::Reading | TossupState::FinishedReading { .. } => {
                let br = c.r(72.0, 540.0, 278.0, 104.0);
                let resp = ui.interact(br, Id::new("qb_buzz"), Sense::click());
                let hover = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
                kit::poly(&painter, kit::pattern_a(br, c.v(16.0)), kit::lerp_color(GOLD, theme::GOLD_HI, hover), Stroke::NONE);
                painter.text(Pos2::new(br.left() + c.v(44.0), br.center().y), Align2::LEFT_CENTER, "BUZZ", c.cond(800, 50.0), ON_GOLD);
                let key = c.r(242.0, 578.0, 64.0, 28.0);
                painter.rect_filled(key, 0.0, theme::alpha(ON_GOLD, 0.2));
                kit::text_tracked(&painter, key.center(), Align2::CENTER_CENTER, "SPACE", c.cond(700, 13.0), ON_GOLD, 0.14);
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                kit::label(&painter, &c, c.p(374.0, 574.0), Align2::LEFT_CENTER, if words_left > 0 { "STILL TO READ" } else { "BUZZ BEFORE TIME RUNS OUT" }, MUTED);
                let left_text = if words_left > 0 {
                    format!("{words_left} WORD{}", if words_left == 1 { "" } else { "S" })
                } else if let TossupState::FinishedReading { elapsed } = self.state {
                    format!("{}S", (5.0 - elapsed).ceil().max(0.0) as i32)
                } else {
                    String::new()
                };
                painter.text(c.p(374.0, 606.0), Align2::LEFT_CENTER, left_text, c.cond(800, 30.0), INK);
                if resp.clicked() || ui.input(|i| i.key_pressed(Key::Space)) {
                    buzz = true;
                }
            }

            TossupState::BuzzedIn { ref mut answer_input } => {
                if let Some(name) = &self.mp_buzzed_name {
                    kit::label(&painter, &c, c.p(72.0, 556.0), Align2::LEFT_CENTER, "BUZZED IN", GOLD);
                    painter.text(c.p(72.0, 596.0), Align2::LEFT_CENTER, format!("{} IS ANSWERING ON THEIR PHONE", name.to_uppercase()), c.cond(800, 36.0), INK);
                } else {
                    kit::label(&painter, &c, c.p(72.0, 532.0), Align2::LEFT_CENTER, "YOUR ANSWER", MUTED);
                    let mut go = kit::answer_field(ui, c.r(72.0, 548.0, 480.0, 64.0), answer_input, self.focus_answer, c.v(30.0));
                    self.focus_answer = false;
                    let sb = c.r(560.0, 548.0, 150.0, 64.0);
                    go |= kit::button(ui, &c, sb, "SUBMIT", Btn::Primary, "qb_submit").clicked();
                    kit::key_hint(&painter, &c, Pos2::new(sb.right() + c.v(12.0), sb.center().y), Align2::LEFT_CENTER, "ENTER ↵");
                    if go {
                        // An empty answer counts as incorrect.
                        submitted = Some(answer_input.clone());
                    }
                }
            }

            TossupState::TimedOut => {
                next = self.verdict(ui, &c, "TIME", BAD, ON_BAD, None, "NEXT TOSSUP");
            }

            TossupState::Result { correct, score_delta } => {
                let (word, fill, ink) = if correct { ("CORRECT", GOOD, ON_GOOD) } else { ("WRONG", BAD, ON_BAD) };
                // A correct tossup earns a bonus round; a miss goes to the next tossup.
                let btn = if correct { "BONUS ROUND" } else { "NEXT TOSSUP" };
                if self.verdict(ui, &c, word, fill, ink, Some(score_delta), btn) {
                    if correct { bonus = true } else { next = true }
                }
            }
        }

        // -- Roster -------------------------------------------------------------------------------
        let reading = matches!(self.state, TossupState::Reading | TossupState::FinishedReading { .. });
        if self.phones.is_empty() {
            let mut e = RosterEntry::new("You", self.score);
            e.status = Some(if reading { "READY" } else if matches!(self.state, TossupState::BuzzedIn { .. }) { "BUZZED" } else { "" });
            if let TossupState::Result { score_delta, .. } = self.state {
                e.delta = Some(score_delta);
            }
            kit::roster(ui, &c, 72.0, 372.0, 772.0, &[e], None, false);
        } else {
            let buzzed = self.mp_buzzed_name.clone();
            let active = buzzed.as_ref().and_then(|b| self.phones.iter().position(|(n, _)| n == b));
            let entries: Vec<RosterEntry> = self
                .phones
                .iter()
                .map(|(n, s)| {
                    let mut e = RosterEntry::new(n, *s);
                    e.status = Some(match &buzzed {
                        Some(b) if b == n => "BUZZED",
                        Some(_) => "LOCKED OUT",
                        None if reading => "READY",
                        None => "",
                    });
                    e
                })
                .collect();
            let w = (entries.len() as f32 * 372.0 + (entries.len() as f32 - 1.0) * 12.0).min(1136.0);
            kit::roster(ui, &c, 72.0, w, 772.0, &entries, active, active.is_some());
        }

        // -- Apply transitions ----------------------------------------------------------------------
        let mut action = None;
        if buzz {
            self.state = TossupState::BuzzedIn { answer_input: String::new() };
            self.focus_answer = true;
        } else if let Some(ans) = submitted {
            let correct = check_answer(&ans, &self.tossup.accepted_answers);
            let delta = quizbowl_score(correct, self.chars_revealed, total_chars);
            self.score += delta;
            self.chars_revealed = total_chars;
            self.state = TossupState::Result { correct, score_delta: delta };
            self.result_start = now;
            self.bump_start = now;
        } else if bonus {
            action = Some(TossupAction::Bonus);
        } else if next {
            action = Some(TossupAction::Next);
        }
        action
    }

    /// Verdict band + answer + advance button. Returns true when advanced.
    #[allow(clippy::too_many_arguments)]
    fn verdict(&self, ui: &mut egui::Ui, c: &Canvas, word: &str, fill: egui::Color32, ink: egui::Color32, delta: Option<i32>, next: &str) -> bool {
        let painter = ui.painter().clone();
        let t = kit::ease_out(kit::since(ui, self.result_start, 0.3) / 0.3);
        let band = c.r(72.0, 540.0, 500.0, 92.0);
        let band = Rect::from_center_size(band.center(), Vec2::new(band.width(), band.height() * t.max(0.02)));
        kit::poly(&painter, kit::pattern_a(band, c.v(theme::CHAMFER_MD)), fill, Stroke::NONE);
        if t > 0.6 {
            painter.text(Pos2::new(band.left() + c.v(24.0), band.center().y), Align2::LEFT_CENTER, word, c.cond(800, 54.0), ink);
            if let Some(d) = delta {
                let txt = if d >= 0 { format!("+{d}") } else { format!("−{}", -d) };
                painter.text(Pos2::new(band.right() - c.v(24.0), band.center().y), Align2::RIGHT_CENTER, txt, c.cond(800, 40.0), ink);
            }
        }
        kit::label(&painter, c, c.p(596.0, 550.0), Align2::LEFT_CENTER, "ANSWER", MUTED);
        kit::wrapped(&painter, c.p(596.0, 564.0), &self.tossup.answer.to_uppercase(), c.cond(800, 28.0), INK, c.v(400.0), c.v(30.0));
        let br = c.r(1016.0, 540.0, 192.0, 64.0);
        kit::key_hint(&painter, c, Pos2::new(br.right(), br.bottom() + c.v(16.0)), Align2::RIGHT_CENTER, "ENTER ↵");
        kit::button(ui, c, br, next, Btn::Primary, "qb_next").clicked() || ui.input(|i| i.key_pressed(Key::Enter))
    }
}
