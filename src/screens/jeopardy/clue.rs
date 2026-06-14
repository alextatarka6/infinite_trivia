use egui::{Align2, Pos2, Rect, Vec2};
use crate::api::jeopardy::{check_answer, Clue};
use crate::screens::jeopardy::board::money;
use crate::screens::kit;
use crate::theme::{self, GREEN_CORRECT, INK_DIM, RED_WRONG};

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
            enter_start: -1.0,
            result_start: -10.0,
            focus_input: true,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<ClueAction> {
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

        let mut y = ((avail.y - 360.0) / 2.0).max(40.0) + ey;

        // -- Meta (.it-clue-meta) ---------------------------------------------
        let cat = self.clue.category.to_uppercase();
        let val = money(self.clue.value);
        let cat_w = painter.layout_no_wrap(cat.clone(), theme::mono(12.0), INK_DIM).size().x;
        let val_w = painter.layout_no_wrap(val.clone(), theme::mono(14.0), INK_DIM).size().x;
        let total = cat_w + 14.0 + val_w;
        let left = cx - total / 2.0;
        painter.text(Pos2::new(left, y), Align2::LEFT_CENTER, &cat, theme::mono(12.0), theme::GOLD);
        painter.text(Pos2::new(left + cat_w + 14.0, y), Align2::LEFT_CENTER, &val, theme::mono(14.0), INK_DIM);
        y += 28.0;

        // -- Question (.it-clue-q) --------------------------------------------
        let qh = kit::centered_question(ui, cx, y, &self.clue.question, 32.0, 720.0_f32.min(avail.x - 80.0));
        y += qh + 30.0;

        match self.state {
            ClueState::ShowQuestion => {
                if kit::input_row(ui, cx, y + 21.0, "What is", &mut self.input, 340.0, self.focus_input) {
                    self.answer(now);
                }
                self.focus_input = false;
                y += 58.0;

                // Submit (primary) + Reveal (ghost), centered pair
                let pw = kit_measure(ui, "Submit") + 44.0;
                let gw = kit_measure(ui, "Reveal") + 44.0;
                let lp = cx - (pw + 14.0 + gw) / 2.0;
                let pr = Rect::from_min_size(Pos2::new(lp, y), Vec2::new(pw, 42.0));
                let gr = Rect::from_min_size(Pos2::new(lp + pw + 14.0, y), Vec2::new(gw, 42.0));
                let mut pc = child(ui, pr);
                if kit::primary_button(&mut pc, "Submit").clicked() {
                    self.answer(now);
                }
                let mut gc = child(ui, gr);
                if kit::ghost_button(&mut gc, "Reveal").clicked() {
                    self.state = ClueState::Answered { correct: false };
                    self.result_start = now;
                }
            }

            ClueState::Answered { correct } => {
                let (label, color) = if correct { ("CORRECT", GREEN_CORRECT) } else { ("INCORRECT", RED_WRONG) };
                let delta = if correct { Some(format!("+{}", money(self.clue.value))) } else { None };
                kit::overlay_verdict(ui, cx, y + 22.0, self.result_start, label, color, delta);
                y += 90.0;
                kit::answer_reveal(ui, cx, y, "CORRECT RESPONSE", &self.clue.answer);
                y += 58.0;
                if kit::centered_primary(ui, cx, y, "Back to board →") {
                    action = Some(ClueAction::Done { correct });
                }
            }
        }

        action
    }

    fn answer(&mut self, now: f64) {
        let correct = check_answer(&self.input, &self.clue.answer);
        self.state = ClueState::Answered { correct };
        self.result_start = now;
    }
}

fn kit_measure(ui: &egui::Ui, label: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(label.to_string(), theme::plex_semibold(14.0), theme::WHITE)
        .size()
        .x
}

fn child(ui: &mut egui::Ui, rect: Rect) -> egui::Ui {
    ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    )
}
