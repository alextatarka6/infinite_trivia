use egui::{Align2, Pos2};
use crate::api::jeopardy::{check_answer, Clue};
use crate::screens::jeopardy::board::money;
use crate::screens::kit;
use crate::theme::{self, ACCENT_SOFT, GOLD, GREEN_CORRECT, INK_DIM, RED_WRONG};

pub enum DailyDoubleAction {
    Done { winnings: i32 },
}

pub struct DailyDoubleScreen {
    pub clue: Clue,
    pub current_score: i32,
    pub wager_input: String,
    pub wager: Option<i32>,
    pub answer_input: String,
    pub answered: Option<bool>,
    enter_start: f64,
    result_start: f64,
    focus_field: bool,
}

impl DailyDoubleScreen {
    pub fn new(clue: Clue, current_score: i32) -> Self {
        Self {
            clue,
            current_score,
            wager_input: String::new(),
            wager: None,
            answer_input: String::new(),
            answered: None,
            enter_start: -1.0,
            result_start: -10.0,
            focus_field: true,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<DailyDoubleAction> {
        kit::ambient(ui.ctx());
        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        let ey = kit::enter_offset(ui, self.enter_start);
        ui.ctx().request_repaint();

        let avail = ui.available_size();
        let cx = avail.x / 2.0;
        let painter = ui.painter().clone();
        let mut action = None;
        let max_wager = self.current_score.max(1000);

        let mut y = ((avail.y - 380.0) / 2.0).max(40.0) + ey;

        // -- Splash (.it-splash) with pulsing halo ----------------------------
        let pulse = 0.5 + 0.5 * ((now as f32) / 2.6 * std::f32::consts::TAU).sin();
        painter.circle_filled(
            Pos2::new(cx, y + 4.0),
            150.0,
            ACCENT_SOFT.gamma_multiply(0.12 + 0.10 * pulse),
        );
        painter.text(Pos2::new(cx, y + 4.0), Align2::CENTER_CENTER, "DAILY DOUBLE!", theme::spectral(52.0), GOLD);
        y += 44.0;
        painter.text(
            Pos2::new(cx, y),
            Align2::CENTER_CENTER,
            self.clue.category.to_uppercase(),
            theme::mono(12.0),
            GOLD,
        );
        y += 28.0;

        if self.wager.is_none() {
            // -- Wager phase --------------------------------------------------
            painter.text(
                Pos2::new(cx, y),
                Align2::CENTER_CENTER,
                format!("SCORE  {}  ·  MAX WAGER  {}", money(self.current_score), money(max_wager)),
                theme::mono(12.0),
                INK_DIM,
            );
            y += 30.0;
            if kit::input_row(ui, cx, y + 21.0, "YOUR WAGER  $", &mut self.wager_input, 160.0, self.focus_field) {
                self.confirm_wager(max_wager);
            }
            self.focus_field = false;
            y += 60.0;
            if kit::centered_primary(ui, cx, y, "Confirm wager →") {
                self.confirm_wager(max_wager);
            }
        } else if self.answered.is_none() {
            // -- Question phase ----------------------------------------------
            let qh = kit::centered_question(ui, cx, y, &self.clue.question, 26.0, 640.0_f32.min(avail.x - 80.0));
            y += qh + 26.0;
            if kit::input_row(ui, cx, y + 21.0, "What is", &mut self.answer_input, 300.0, self.focus_field) {
                self.submit(now);
            }
            self.focus_field = false;
            y += 60.0;
            if kit::centered_primary(ui, cx, y, "Submit") {
                self.submit(now);
            }
        } else {
            // -- Result phase -------------------------------------------------
            let correct = self.answered.unwrap();
            let wager = self.wager.unwrap();
            let (label, color) = if correct { ("CORRECT", GREEN_CORRECT) } else { ("INCORRECT", RED_WRONG) };
            let delta = Some(format!("{}{}", if correct { "+" } else { "-" }, money(wager)));
            kit::overlay_verdict(ui, cx, y + 22.0, self.result_start, label, color, delta);
            y += 90.0;
            kit::answer_reveal(ui, cx, y, "CORRECT RESPONSE", &self.clue.answer);
            y += 58.0;
            if kit::centered_primary(ui, cx, y, "Back to board →") {
                action = Some(DailyDoubleAction::Done {
                    winnings: if correct { wager } else { -wager },
                });
            }
        }

        action
    }

    fn confirm_wager(&mut self, max_wager: i32) {
        if let Ok(w) = self.wager_input.trim().parse::<i32>() {
            self.wager = Some(w.clamp(5, max_wager));
            self.focus_field = true;
        }
    }

    fn submit(&mut self, now: f64) {
        let correct = check_answer(&self.answer_input, &self.clue.answer);
        self.answered = Some(correct);
        self.result_start = now;
    }
}
