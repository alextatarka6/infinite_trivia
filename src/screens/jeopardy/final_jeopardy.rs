use egui::{Align2, Pos2, Rect, Rounding, Stroke, Vec2};
use crate::api::jeopardy::{check_answer, Clue};
use crate::screens::jeopardy::board::money;
use crate::screens::kit;
use crate::theme::{self, BLUE_MID, GOLD, GREEN_CORRECT, INK_DIM, LINE, RED_WRONG, WHITE};

pub enum FinalAction {
    Done { score_delta: i32 },
}

pub struct FinalJeopardy {
    pub clue: Clue,
    pub current_score: i32,
    pub wager_input: String,
    pub wager: Option<i32>,
    pub answer_input: String,
    pub answered: Option<bool>,
    pub show_category: bool,
    enter_start: f64,
    result_start: f64,
    focus_field: bool,
}

impl FinalJeopardy {
    pub fn new(clue: Clue, current_score: i32) -> Self {
        Self {
            clue,
            current_score,
            wager_input: String::new(),
            wager: None,
            answer_input: String::new(),
            answered: None,
            show_category: true,
            enter_start: -1.0,
            result_start: -10.0,
            focus_field: true,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<FinalAction> {
        kit::ambient(ui.ctx());
        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        let ey = kit::enter_offset(ui, self.enter_start);

        let avail = ui.available_size();
        let cx = avail.x / 2.0;
        let painter = ui.painter().clone();
        let mut action = None;
        let max_wager = self.current_score.max(0);

        let mut y = ((avail.y - 400.0) / 2.0).max(36.0) + ey;

        // -- Title (.it-fj-title) ---------------------------------------------
        painter.text(Pos2::new(cx, y), Align2::CENTER_CENTER, "FINAL JEOPARDY", theme::spectral(38.0), GOLD);
        y += 30.0;

        // -- Category label + card (.it-fj-cat-label / .it-cat-card) ----------
        painter.text(Pos2::new(cx, y), Align2::CENTER_CENTER, "CATEGORY", theme::mono(10.5), INK_DIM);
        y += 18.0;
        let cat = self.clue.category.to_uppercase();
        let cat_galley = painter.layout_no_wrap(cat.clone(), theme::spectral_medium(30.0), WHITE);
        let card_w = cat_galley.size().x + 88.0;
        let card_h = cat_galley.size().y + 40.0;
        let card = Rect::from_center_size(Pos2::new(cx, y + card_h / 2.0), Vec2::new(card_w, card_h));
        painter.rect_filled(card, Rounding::same(theme::RADIUS_CARD), BLUE_MID);
        painter.rect_stroke(card, Rounding::same(theme::RADIUS_CARD), Stroke::new(1.0, LINE));
        painter.text(card.center(), Align2::CENTER_CENTER, &cat, theme::spectral_medium(30.0), WHITE);
        y += card_h + 24.0;

        if self.show_category {
            // -- Wager phase --------------------------------------------------
            painter.text(
                Pos2::new(cx, y),
                Align2::CENTER_CENTER,
                format!("SCORE  {}  ·  MAX WAGER  {}", money(self.current_score), money(max_wager)),
                theme::mono(12.0),
                INK_DIM,
            );
            y += 30.0;
            if kit::input_row(ui, cx, y + 21.0, "WAGER  $", &mut self.wager_input, 160.0, self.focus_field) {
                self.lock_wager(max_wager);
            }
            self.focus_field = false;
            y += 60.0;
            if kit::centered_primary(ui, cx, y, "Lock wager →") {
                self.lock_wager(max_wager);
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
            let wager = self.wager.unwrap_or(0);
            let delta = if correct { wager } else { -wager };
            let (label, color) = if correct { ("CORRECT", GREEN_CORRECT) } else { ("INCORRECT", RED_WRONG) };
            let pop = Some(format!("{}{}", if correct { "+" } else { "-" }, money(wager)));
            kit::overlay_verdict(ui, cx, y + 18.0, self.result_start, label, color, pop);
            y += 86.0;
            painter.text(
                Pos2::new(cx, y),
                Align2::CENTER_CENTER,
                format!("Final score: {}", money(self.current_score + delta)),
                theme::spectral_medium(20.0),
                GOLD,
            );
            y += 34.0;
            kit::answer_reveal(ui, cx, y, "CORRECT RESPONSE", &self.clue.answer);
            y += 56.0;
            if kit::centered_primary(ui, cx, y, "Return to menu") {
                action = Some(FinalAction::Done { score_delta: delta });
            }
        }

        action
    }

    fn lock_wager(&mut self, max_wager: i32) {
        if let Ok(w) = self.wager_input.trim().parse::<i32>() {
            self.wager = Some(w.clamp(0, max_wager));
            self.show_category = false;
            self.focus_field = true;
        }
    }

    fn submit(&mut self, now: f64) {
        self.answered = Some(check_answer(&self.answer_input, &self.clue.answer));
        self.result_start = now;
    }
}
