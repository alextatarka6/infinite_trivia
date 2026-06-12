use egui::{Align2, Pos2};
use crate::api::qbreader::{check_answer, Bonus};
use crate::screens::kit;
use crate::theme::{self, GOLD, GREEN_CORRECT, INK_DIM, RED_WRONG, WHITE};

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
        kit::ambient(ui.ctx());

        if self.loading {
            kit::loader(ui, "Fetching a bonus…");
            return None;
        }

        if let Some(err) = self.error.clone() {
            let mut action = None;
            let avail = ui.available_size();
            ui.add_space((avail.y - 120.0).max(16.0) / 2.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("Could not fetch bonus").font(theme::spectral(26.0)).color(RED_WRONG));
                ui.add_space(8.0);
                ui.label(egui::RichText::new(&err).font(theme::plex(14.0)).color(INK_DIM));
                ui.add_space(18.0);
                if kit::primary_button(ui, "Continue →").clicked() {
                    action = Some(BonusAction::Done { score: self.score });
                }
            });
            return action;
        }

        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        if kit::quit_button(ui) {
            return Some(BonusAction::Quit);
        }
        let ey = kit::enter_offset(ui, self.enter_start);

        let avail = ui.available_size();
        let cx = avail.x / 2.0;
        let content_w = (avail.x - 120.0).min(780.0).max(420.0);
        let left = (avail.x - content_w) / 2.0;
        let right = left + content_w;
        let painter = ui.painter().clone();
        let n_parts = self.bonus.parts.len().max(1);
        let mut action = None;

        // -- Top bar ----------------------------------------------------------
        let bar_cy = 26.0 + ey;
        painter.text(
            Pos2::new(left, bar_cy),
            Align2::LEFT_CENTER,
            format!(
                "BONUS   •   {}   •   {}",
                self.bonus.category.to_uppercase(),
                self.bonus.difficulty.to_uppercase()
            ),
            theme::mono(11.0),
            GOLD,
        );
        let bump = {
            let e = (now - self.bump_start) as f32;
            if e < 0.5 { (e / 0.5 * std::f32::consts::PI).sin() } else { 0.0 }
        };
        kit::score_chip(
            ui,
            Pos2::new(right, bar_cy),
            &format!("Q{}", self.questions_done + 1),
            &format!("{} pts", self.score),
            bump,
        );

        // -- Readout: leadin + PART kicker + part text, vertically centered ---
        // (mirrors the tossup readout: a large centered block with the footer
        // pinned to the bottom).
        let screen = ui.ctx().screen_rect();
        let foot_bottom = screen.bottom() - 28.0 + ey;
        let region_top = 64.0 + ey;
        let region_bottom = foot_bottom - 185.0;

        let leadin = para(ui, &self.bonus.leadin, theme::spectral_medium(24.0), WHITE, content_w);
        let part_g = para(
            ui,
            &self.bonus.parts[self.part_idx].text,
            theme::spectral_medium(28.0),
            WHITE,
            content_w,
        );
        let kicker_h = 22.0;
        let (gap1, gap2) = (16.0, 10.0);
        let total_h = leadin.size().y + gap1 + kicker_h + gap2 + part_g.size().y;
        let mut ry = ((region_top + region_bottom - total_h) / 2.0).max(region_top);

        painter.galley(Pos2::new(cx, ry), leadin.clone(), WHITE);
        ry += leadin.size().y + gap1;
        painter.text(
            Pos2::new(cx, ry + kicker_h / 2.0),
            Align2::CENTER_CENTER,
            format!("PART {} / {}", self.part_idx + 1, n_parts),
            theme::mono(11.0),
            GOLD,
        );
        ry += kicker_h + gap2;
        painter.galley(Pos2::new(cx, ry), part_g, WHITE);

        // -- Footer (pinned to bottom) ----------------------------------------
        match &mut self.phase {
            Phase::Answering { input } => {
                if kit::input_row(ui, cx, foot_bottom - 95.0, "Answer", input, 360.0, self.focus_input) {
                    self.submit(now);
                }
                self.focus_input = false;
                if kit::centered_primary(ui, cx, foot_bottom - 44.0, "Submit") {
                    self.submit(now);
                }
            }
            Phase::Revealed { correct } => {
                let correct = *correct;
                let (label, color) = if correct { ("CORRECT", GREEN_CORRECT) } else { ("INCORRECT", RED_WRONG) };
                let delta = Some(format!("{}{}", if correct { "+" } else { "" }, if correct { PART_VALUE } else { 0 }));
                kit::overlay_verdict(ui, cx, foot_bottom - 165.0, self.result_start, label, color, delta);
                kit::answer_reveal(ui, cx, foot_bottom - 100.0, "ANSWER", &self.bonus.parts[self.part_idx].answer);

                // A wrong answer ends the bonus (in multiplayer the remaining
                // parts would open to other players); otherwise advance until
                // the last part is answered.
                let ended = !correct || self.part_idx + 1 >= n_parts;
                let btn = if ended { "Continue →" } else { "Next part →" };
                if kit::centered_primary(ui, cx, foot_bottom - 44.0, btn) {
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
            self.phase = Phase::Revealed { correct };
            self.result_start = now;
        }
    }
}

/// Lay out centered, wrapped text and return the galley (so its height can be
/// measured for vertical centering before drawing).
fn para(
    ui: &egui::Ui,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
    max_w: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.halign = egui::Align::Center;
    job.wrap.max_width = max_w;
    let line_height = font.size * 1.34;
    job.append(
        text,
        0.0,
        egui::TextFormat { font_id: font, color, line_height: Some(line_height), ..Default::default() },
    );
    ui.fonts(|f| f.layout_job(job))
}
