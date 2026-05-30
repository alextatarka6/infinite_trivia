use egui::RichText;
use crate::api::jeopardy::{check_answer, Clue};
use crate::theme;

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
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<DailyDoubleAction> {
        let mut action = None;
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(
                RichText::new("DAILY DOUBLE!")
                    .font(theme::heading_font())
                    .color(theme::GOLD),
            );
            ui.add_space(12.0);

            if self.wager.is_none() {
                let max_wager = self.current_score.max(1000);
                ui.label(
                    RichText::new(format!(
                        "Current score: ${} | Max wager: ${}",
                        self.current_score, max_wager
                    ))
                    .color(theme::WHITE),
                );
                ui.add_space(8.0);
                ui.label(RichText::new("Enter wager:").color(theme::GOLD));
                let resp = ui.text_edit_singleline(&mut self.wager_input);
                let submitted = resp.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if submitted
                    || ui.button(RichText::new("Confirm Wager").color(theme::BLUE_BG)).clicked()
                {
                    if let Ok(w) = self.wager_input.trim().parse::<i32>() {
                        let clamped = w.clamp(5, max_wager);
                        self.wager = Some(clamped);
                    }
                }
            } else if self.answered.is_none() {
                ui.label(
                    RichText::new(&self.clue.question)
                        .font(theme::subheading_font())
                        .color(theme::WHITE),
                );
                ui.add_space(8.0);
                ui.label(RichText::new("What is…?").color(theme::GOLD));
                let resp = ui.text_edit_singleline(&mut self.answer_input);
                let submitted = resp.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if submitted
                    || ui.button(RichText::new("Submit").color(theme::BLUE_BG)).clicked()
                {
                    let correct = check_answer(&self.answer_input, &self.clue.answer);
                    self.answered = Some(correct);
                }
            } else {
                let correct = self.answered.unwrap();
                let wager = self.wager.unwrap();
                let winnings = if correct { wager } else { -wager };
                let (label, color) = if correct {
                    ("Correct!", theme::GREEN_CORRECT)
                } else {
                    ("Incorrect", theme::RED_WRONG)
                };
                ui.label(RichText::new(label).font(theme::heading_font()).color(color));
                ui.label(
                    RichText::new(format!("Answer: {}", self.clue.answer))
                        .color(theme::WHITE),
                );
                ui.add_space(16.0);
                if ui
                    .button(RichText::new("Back to Board").color(theme::BLUE_BG))
                    .clicked()
                {
                    action = Some(DailyDoubleAction::Done { winnings });
                }
            }
        });
        action
    }
}
