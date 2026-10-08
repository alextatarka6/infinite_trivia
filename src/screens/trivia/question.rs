//! General Trivia — "05 Trivia" in the Roster hi-fi.

use egui::{Align2, Id, Key, Pos2, Rect, Sense, Stroke, Vec2};
use crate::api::opentdb::TriviaQuestion;
use crate::game::scoring::trivia_score;
use crate::screens::kit::{self, Btn, Canvas, RosterEntry, SummaryAction, Texture};
use crate::theme::{self, BAD, GOLD, GOOD, INK, MUTED, ON_BAD, ON_GOOD, PANEL, PANEL_HI};

pub enum TriviaAction {
    Next { score_delta: i32 },
    Restart,
    Quit,
}

pub struct TriviaScreen {
    pub questions: Vec<TriviaQuestion>,
    pub current_idx: usize,
    pub score: i32,
    pub streak: u32,
    pub best_streak: u32,
    pub correct_count: u32,
    pub selected: Option<usize>,
    pub timer: f32,
    pub timer_max: f32,
    pub loading: bool,
    pub error: Option<String>,
    /// Phone players' (name, score), set by App during a phone game so the
    /// roster shows the couch instead of a lone YOU card.
    pub phones: Vec<(String, i32)>,
    // animation bookkeeping
    anim_idx: i64,
    enter_start: f64,
    select_start: f64,
    bump_start: f64,
}

impl TriviaScreen {
    pub fn new(questions: Vec<TriviaQuestion>) -> Self {
        Self {
            questions,
            current_idx: 0,
            score: 0,
            streak: 0,
            best_streak: 0,
            correct_count: 0,
            selected: None,
            timer: 20.0,
            timer_max: 20.0,
            loading: false,
            error: None,
            phones: Vec::new(),
            anim_idx: -1,
            enter_start: 0.0,
            select_start: 0.0,
            bump_start: -10.0,
        }
    }

    pub fn loading() -> Self {
        let mut s = Self::new(vec![]);
        s.loading = true;
        s
    }

    pub fn with_error(msg: String) -> Self {
        let mut s = Self::new(vec![]);
        s.error = Some(msg);
        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, dt: f32) -> Option<TriviaAction> {
        kit::backdrop(ui.ctx(), Some(Texture::Hatch));

        if self.loading {
            kit::loading(ui, "PULLING 10 QUESTIONS", "GENERAL TRIVIA · OPEN TRIVIA DB", None);
            return None;
        }
        if let Some(err) = self.error.clone() {
            return error_screen(ui, "COULD NOT FETCH QUESTIONS", &err).then_some(TriviaAction::Quit);
        }

        let now = ui.input(|i| i.time);
        if self.anim_idx != self.current_idx as i64 {
            self.anim_idx = self.current_idx as i64;
            self.enter_start = now;
        }

        // -- Round complete ---------------------------------------------------------
        if self.current_idx >= self.questions.len() && !self.questions.is_empty() {
            let misses = (self.questions.len() as u32).saturating_sub(self.correct_count);
            let stats = [
                kit::Stat { value: format!("{}/{}", self.correct_count, self.questions.len()), label: "correct" },
                kit::Stat { value: misses.to_string(), label: "misses" },
                kit::Stat { value: format!("×{}", self.best_streak), label: "best streak" },
            ];
            return match kit::summary(ui, "General Trivia", &self.score.to_string(), "", &stats, self.enter_start) {
                SummaryAction::Restart => Some(TriviaAction::Restart),
                SummaryAction::Quit => Some(TriviaAction::Quit),
                SummaryAction::None => None,
            };
        }
        if self.questions.is_empty() {
            return None;
        }
        if kit::quit_button(ui) {
            return Some(TriviaAction::Quit);
        }

        // Tick the countdown.
        if self.selected.is_none() {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.timer = 0.0;
                self.streak = 0;
                self.selected = Some(usize::MAX);
                self.select_start = now;
            }
        }

        let c = Canvas::new(ui.ctx());
        let painter = ui.painter().clone();
        let q = &self.questions[self.current_idx];
        let answers = q.shuffled_answers();
        let enter = kit::ease_out(kit::since(ui, self.enter_start, 0.4) / 0.4);
        let ey = c.v(12.0) * (1.0 - enter);

        // -- Top bar: Q counter, category, skew-tick timer ---------------------------------
        let qn = painter.text(c.p(64.0, 58.0), Align2::LEFT_CENTER, format!("{:02}", self.current_idx + 1), c.cond(800, 24.0), GOLD);
        let tot = painter.text(Pos2::new(qn.right() + c.v(4.0), qn.center().y), Align2::LEFT_CENTER,
            format!("/ {}", self.questions.len()), c.cond(800, 24.0), MUTED);
        kit::label(&painter, &c, Pos2::new(tot.right() + c.v(16.0), qn.center().y), Align2::LEFT_CENTER, &q.category.to_uppercase(), MUTED);
        let ratio = (self.timer / self.timer_max).clamp(0.0, 1.0);
        kit::skew_ticks(&painter, c.r(760.0, 47.0, 300.0, 22.0), 20, ratio, c.v(4.0));
        painter.text(c.p(1100.0, 58.0), Align2::RIGHT_CENTER, format!("{}", self.timer.ceil() as i32),
            c.cond(800, 26.0), if ratio > 0.25 { GOLD } else { BAD });

        // -- Question --------------------------------------------------------------------------
        let text = q.question.to_uppercase();
        let max_w = c.v(1060.0);
        let size = kit::fit_size(&painter, &text, 600, c.v(theme::TYPE_CLUE), c.v(28.0), max_w, c.v(160.0), 1.12);
        kit::wrapped(&painter, c.p(64.0, 118.0) + Vec2::new(0.0, ey), &text, theme::cond(600, size), INK, max_w, size * 1.12);

        // -- 2×2 answers -------------------------------------------------------------------------
        let locked = self.selected.is_some();
        let mut clicked = None;
        if !locked && !ui.ctx().wants_keyboard_input() {
            ui.input(|i| {
                for (k, keys) in [[Key::A, Key::Num1], [Key::B, Key::Num2], [Key::C, Key::Num3], [Key::D, Key::Num4]].iter().enumerate() {
                    if k < answers.len() && keys.iter().any(|key| i.key_pressed(*key)) {
                        clicked = Some(k);
                    }
                }
            });
        }
        for (i, (ans, is_correct)) in answers.iter().enumerate() {
            let (col, row) = (i % 2, i / 2);
            let rect = c.r(64.0 + col as f32 * 581.0, 312.0 + row as f32 * 94.0, 571.0, 84.0).translate(Vec2::new(0.0, ey));
            let resp = ui.interact(rect, Id::new(("trivia_opt", self.current_idx, i)), if locked { Sense::hover() } else { Sense::click() });
            let hover = ui.ctx().animate_bool_with_time(resp.id, resp.hovered() && !locked, 0.12);
            let picked = self.selected == Some(i);
            let (fill, ink, badge, badge_ink, alpha, note) = match self.selected {
                Some(_) if *is_correct => (GOOD, ON_GOOD, ON_GOOD, GOOD, 1.0, Some(("CORRECT", ON_GOOD))),
                Some(_) if picked => (PANEL, INK, BAD, ON_BAD, 1.0, Some(("YOU PICKED THIS", BAD))),
                Some(_) => (PANEL, INK, PANEL_HI, MUTED, 0.45, None),
                None => (kit::lerp_color(PANEL, PANEL_HI, hover), INK, PANEL_HI, MUTED, 1.0, None),
            };
            let pts = kit::pattern_a(rect, c.v(theme::CHAMFER_MD));
            kit::poly(&painter, pts.clone(), fill.gamma_multiply(alpha), Stroke::NONE);
            if locked && picked && !is_correct {
                kit::inner_stroke(&painter, &pts, c.v(3.0), BAD);
            }
            let b = Rect::from_min_size(Pos2::new(rect.left() + c.v(22.0), rect.center().y - c.v(20.0)), Vec2::splat(c.v(40.0)));
            painter.rect_filled(b, 0.0, badge.gamma_multiply(alpha));
            painter.text(b.center(), Align2::CENTER_CENTER, ((b'A' + i as u8) as char).to_string(), c.cond(800, 18.0), badge_ink.gamma_multiply(alpha));
            let note_w = note.map(|(t, _)| kit::text_width(&painter, t, c.cond(700, 13.0), 0.2) + c.v(24.0)).unwrap_or(0.0);
            let tw = rect.right() - b.right() - c.v(34.0) - note_w;
            let ts = kit::fit_size(&painter, ans, 700, c.v(26.0), c.v(15.0), tw, c.v(64.0), 1.1);
            let galley = {
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = tw;
                job.append(ans, 0.0, egui::TextFormat { font_id: theme::cond(700, ts), color: ink.gamma_multiply(alpha), line_height: Some(ts * 1.1), ..Default::default() });
                painter.layout_job(job)
            };
            painter.galley(Pos2::new(b.right() + c.v(18.0), rect.center().y - galley.size().y / 2.0), galley, INK);
            if let Some((t, col)) = note {
                kit::text_tracked(&painter, Pos2::new(rect.right() - c.v(22.0), rect.center().y), Align2::RIGHT_CENTER, t, c.cond(700, 13.0), col, 0.2);
            }
            if resp.hovered() && !locked {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                clicked = Some(i);
            }
        }

        if let Some(i) = clicked.filter(|_| !locked) {
            let is_correct = answers[i].1;
            self.selected = Some(i);
            self.select_start = now;
            self.bump_start = now;
            let delta = trivia_score(is_correct, self.streak);
            self.score += delta;
            if is_correct {
                self.streak += 1;
                self.correct_count += 1;
                self.best_streak = self.best_streak.max(self.streak);
            } else {
                self.streak = 0;
            }
        }

        // -- Next ------------------------------------------------------------------------------
        let mut action = None;
        let reveal = self.selected.map(|sel| sel != usize::MAX && answers[sel].1);
        if let Some(sel) = self.selected {
            let label = if self.current_idx + 1 < self.questions.len() { "NEXT" } else { "FINISH" };
            if sel == usize::MAX {
                kit::text_tracked(&painter, c.p(64.0, 540.0), Align2::LEFT_CENTER, "TIME'S UP", c.cond(800, 28.0), BAD, 0.06);
            }
            let br = c.r(1066.0, 520.0, 150.0, 56.0);
            kit::key_hint(&painter, &c, Pos2::new(br.left() - c.v(12.0), br.center().y), Align2::RIGHT_CENTER, "ENTER ↵");
            if kit::button(ui, &c, br, label, Btn::Primary, "trivia_next").clicked()
                || (ui.input(|i| i.key_pressed(Key::Enter)) && kit::since(ui, self.select_start, 0.0) > 0.2)
            {
                self.current_idx += 1;
                self.selected = None;
                self.timer = self.timer_max;
                action = Some(TriviaAction::Next { score_delta: 0 });
            }
        }

        // -- Roster ------------------------------------------------------------------------------
        if self.phones.is_empty() {
            let mut e = RosterEntry::new("You", self.score);
            if self.streak > 1 {
                e.note = Some(format!("×{} STREAK", self.streak));
            } else if reveal == Some(false) || self.selected == Some(usize::MAX) {
                e.note = Some("STREAK LOST".into());
            }
            if let Some(ok) = reveal {
                e.delta = Some(if ok { trivia_score(true, self.streak.saturating_sub(1)) } else { 0 });
                e.top = Some(if ok { GOOD } else { BAD });
            }
            kit::roster(ui, &c, 64.0, 376.0, 772.0, &[e], None, false);
        } else {
            let entries: Vec<RosterEntry> = self.phones.iter().map(|(n, s)| RosterEntry::new(n, *s)).collect();
            let w = (entries.len() as f32 * 376.0 + (entries.len() as f32 - 1.0) * 12.0).min(1152.0);
            kit::roster(ui, &c, 64.0, w, 772.0, &entries, None, false);
        }
        action
    }
}

/// Error state shared by the trivia-style screens. Returns true on BACK.
pub fn error_screen(ui: &mut egui::Ui, title: &str, detail: &str) -> bool {
    let c = Canvas::new(ui.ctx());
    let p = ui.painter().clone();
    kit::tag(&p, &c, c.p(64.0, 260.0), "ERROR", kit::TagStyle::Gold, theme::TYPE_TAG);
    p.text(c.p(64.0, 320.0), Align2::LEFT_TOP, title, c.cond(800, 56.0), INK);
    kit::wrapped(&p, c.p(64.0, 392.0), detail, c.barlow(500, theme::TYPE_BODY), MUTED, c.v(800.0), c.v(26.0));
    kit::button(ui, &c, c.r(64.0, 470.0, 200.0, 56.0), "BACK TO MENU", Btn::Primary, "err_back").clicked()
        || ui.input(|i| i.key_pressed(Key::Escape))
}
