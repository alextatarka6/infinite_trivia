//! Quizbowl bonus round — the tossup layout with part chips.

use egui::{Align2, Key, Pos2, Rect, Stroke, TextFormat, Vec2};
use egui::text::LayoutJob;
use crate::api::qbreader::{check_answer, Bonus};
use crate::screens::kit::{self, Btn, Canvas, RosterEntry, TagStyle, Texture};
use crate::screens::trivia::question::error_screen;
use crate::theme::{self, BAD, GOLD, GOOD, INK, MUTED, ON_BAD, ON_GOLD, ON_GOOD, PANEL};

const PART_VALUE: i32 = 10;

pub enum BonusAction {
    /// Bonus finished; carry the running score back to the tossup loop.
    Done { score: i32 },
    Quit,
}

enum Phase {
    Answering { input: String },
    Revealed { correct: bool },
}

pub struct BonusScreen {
    pub bonus: Bonus,
    pub score: i32,
    pub questions_done: u32,
    pub loading: bool,
    pub error: Option<String>,
    part_idx: usize,
    phase: Phase,
    bonus_points: i32,
    /// Correct/incorrect per answered part, for the part chips.
    results: Vec<bool>,
    // animation bookkeeping
    enter_start: f64,
    result_start: f64,
    bump_start: f64,
    focus_input: bool,
}

impl BonusScreen {
    pub fn new(bonus: Bonus, score: i32, questions_done: u32) -> Self {
        Self {
            bonus,
            score,
            questions_done,
            loading: false,
            error: None,
            part_idx: 0,
            phase: Phase::Answering { input: String::new() },
            bonus_points: 0,
            results: Vec::new(),
            enter_start: -1.0,
            result_start: -10.0,
            bump_start: -10.0,
            focus_input: true,
        }
    }

    pub fn loading(score: i32, questions_done: u32) -> Self {
        let dummy = Bonus { leadin: String::new(), parts: vec![], category: String::new(), difficulty: String::new() };
        let mut s = Self::new(dummy, score, questions_done);
        s.loading = true;
        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<BonusAction> {
        kit::backdrop(ui.ctx(), Some(Texture::Scan));
        if self.loading {
            kit::loading(ui, "FETCHING A BONUS", "QUIZBOWL · BONUS ROUND", None);
            return None;
        }
        if let Some(err) = self.error.clone() {
            return error_screen(ui, "COULD NOT FETCH A BONUS", &err).then_some(BonusAction::Done { score: self.score });
        }
        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        if kit::quit_button(ui) {
            return Some(BonusAction::Quit);
        }
        let c = Canvas::new(ui.ctx());
        let painter = ui.painter().clone();
        let n_parts = self.bonus.parts.len().max(1);
        let mut action = None;

        // -- Top bar: BONUS tag, category, part chips --------------------------------
        let tag = kit::tag(&painter, &c, c.p(72.0, 44.0), "BONUS", TagStyle::Gold, theme::TYPE_TAG);
        kit::label(&painter, &c, Pos2::new(tag.right() + c.v(16.0), tag.center().y), Align2::LEFT_CENTER,
            &format!("{} · +{} PER PART", self.bonus.category.to_uppercase(), PART_VALUE), MUTED);
        for i in 0..n_parts {
            let r = c.r(1130.0 - (n_parts - i) as f32 * 90.0, 44.0, 86.0, 36.0);
            let (fill, ink) = match self.results.get(i) {
                Some(true) => (GOOD, ON_GOOD),
                Some(false) => (BAD, ON_BAD),
                None if i == self.part_idx => (GOLD, ON_GOLD),
                None => (PANEL, MUTED),
            };
            painter.rect_filled(r, 0.0, fill);
            kit::text_tracked(&painter, r.center(), Align2::CENTER_CENTER, &format!("PART {}", i + 1), c.cond(800, 15.0), ink, 0.06);
        }

        // -- Lead-in (muted) then the current part ------------------------------------
        let max_w = c.v(990.0);
        let lead = para(&painter, &self.bonus.leadin, theme::barlow(500, c.v(24.0)), MUTED, max_w);
        let lead_h = lead.size().y;
        painter.galley(c.p(72.0, 120.0), lead, MUTED);
        let py = c.p(72.0, 120.0).y + lead_h + c.v(24.0);
        kit::label(&painter, &c, Pos2::new(c.p(72.0, 0.0).x, py), Align2::LEFT_CENTER, &format!("PART {} OF {}", self.part_idx + 1, n_parts), GOLD);
        let part = para(&painter, &self.bonus.parts[self.part_idx].text, theme::barlow(600, c.v(theme::TYPE_QB_TEXT)), INK, max_w);
        painter.galley(Pos2::new(c.p(72.0, 0.0).x, py + c.v(16.0)), part, INK);

        // -- Footer ---------------------------------------------------------------------
        match &mut self.phase {
            Phase::Answering { input } => {
                kit::label(&painter, &c, c.p(72.0, 532.0), Align2::LEFT_CENTER, "YOUR ANSWER", MUTED);
                let mut go = kit::answer_field(ui, c.r(72.0, 548.0, 480.0, 64.0), input, self.focus_input, c.v(30.0));
                self.focus_input = false;
                let sb = c.r(560.0, 548.0, 150.0, 64.0);
                go |= kit::button(ui, &c, sb, "SUBMIT", Btn::Primary, "bonus_submit").clicked();
                kit::key_hint(&painter, &c, Pos2::new(sb.right() + c.v(12.0), sb.center().y), Align2::LEFT_CENTER, "ENTER ↵");
                if go {
                    self.submit(now);
                }
            }
            Phase::Revealed { correct } => {
                let correct = *correct;
                let t = kit::ease_out(kit::since(ui, self.result_start, 0.3) / 0.3);
                let band = c.r(72.0, 540.0, 500.0, 92.0);
                let band = Rect::from_center_size(band.center(), Vec2::new(band.width(), band.height() * t.max(0.02)));
                let (fill, ink, word) = if correct { (GOOD, ON_GOOD, "CORRECT") } else { (BAD, ON_BAD, "WRONG") };
                kit::poly(&painter, kit::pattern_a(band, c.v(theme::CHAMFER_MD)), fill, Stroke::NONE);
                if t > 0.6 {
                    painter.text(Pos2::new(band.left() + c.v(24.0), band.center().y), Align2::LEFT_CENTER, word, c.cond(800, 54.0), ink);
                    let d = if correct { format!("+{PART_VALUE}") } else { "+0".into() };
                    painter.text(Pos2::new(band.right() - c.v(24.0), band.center().y), Align2::RIGHT_CENTER, d, c.cond(800, 40.0), ink);
                }
                kit::label(&painter, &c, c.p(596.0, 550.0), Align2::LEFT_CENTER, "ANSWER", MUTED);
                kit::wrapped(&painter, c.p(596.0, 564.0), &self.bonus.parts[self.part_idx].answer.to_uppercase(), c.cond(800, 28.0), INK, c.v(400.0), c.v(30.0));

                // A wrong answer ends the bonus (in multiplayer the remaining
                // parts would open to other players); otherwise advance until
                // the last part is answered.
                let ended = !correct || self.part_idx + 1 >= n_parts;
                let br = c.r(1016.0, 540.0, 192.0, 64.0);
                kit::key_hint(&painter, &c, Pos2::new(br.right(), br.bottom() + c.v(16.0)), Align2::RIGHT_CENTER, "ENTER ↵");
                if kit::button(ui, &c, br, if ended { "NEXT TOSSUP" } else { "NEXT PART" }, Btn::Primary, "bonus_next").clicked()
                    || ui.input(|i| i.key_pressed(Key::Enter))
                {
                    if ended {
                        action = Some(BonusAction::Done { score: self.score });
                    } else {
                        self.part_idx += 1;
                        self.phase = Phase::Answering { input: String::new() };
                        self.focus_input = true;
                    }
                }
            }
        }

        let mut e = RosterEntry::new("You", self.score);
        e.note = Some(format!("BONUS +{}", self.bonus_points));
        kit::roster(ui, &c, 72.0, 372.0, 772.0, &[e], None, false);
        action
    }

    fn submit(&mut self, now: f64) {
        if let Phase::Answering { input } = &self.phase {
            let part = &self.bonus.parts[self.part_idx];
            let correct = check_answer(input, &part.accepted_answers);
            if correct {
                self.score += PART_VALUE;
                self.bonus_points += PART_VALUE;
                self.bump_start = now;
            }
            self.results.push(correct);
            self.phase = Phase::Revealed { correct };
            self.result_start = now;
        }
    }
}

/// Left-aligned wrapped paragraph, returned as a galley so it can be measured.
fn para(painter: &egui::Painter, text: &str, font: egui::FontId, color: egui::Color32, max_w: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::default();
    job.wrap.max_width = max_w;
    let line_height = font.size * 1.38;
    job.append(text, 0.0, TextFormat { font_id: font, color, line_height: Some(line_height), ..Default::default() });
    painter.layout_job(job)
}
