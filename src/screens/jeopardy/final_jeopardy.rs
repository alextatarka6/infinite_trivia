use egui::RichText;
use crate::api::jeopardy::{check_answer, Clue};
use crate::theme;

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
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<FinalAction> {
        let mut action = None;
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(
                RichText::new("FINAL JEOPARDY!")
                    .font(theme::heading_font())
                    .color(theme::GOLD),
            );
            ui.add_space(8.0);

            if self.show_category {
                ui.label(
                    RichText::new(format!("Category: {}", self.clue.category.to_uppercase()))
                        .font(theme::subheading_font())
                        .color(theme::WHITE),
                );
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("Score: ${}", self.current_score)).color(theme::WHITE),
                );
                ui.label(RichText::new("Enter wager:").color(theme::GOLD));
                let resp = ui.text_edit_singleline(&mut self.wager_input);
                let submitted = resp.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if submitted
                    || ui.button(RichText::new("Reveal Question").color(theme::BLUE_BG)).clicked()
                {
                    if let Ok(w) = self.wager_input.trim().parse::<i32>() {
                        let max = self.current_score.max(0);
                        self.wager = Some(w.clamp(0, max));
                        self.show_category = false;
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
                let wager = self.wager.unwrap_or(0);
                let delta = if correct { wager } else { -wager };
                let (label, color) = if correct {
                    ("Correct!", theme::GREEN_CORRECT)
                } else {
                    ("Incorrect", theme::RED_WRONG)
                };
                ui.label(RichText::new(label).font(theme::heading_font()).color(color));
                ui.label(
                    RichText::new(format!("Answer: {}", self.clue.answer)).color(theme::WHITE),
                );
                ui.label(
                    RichText::new(format!(
                        "Final score: ${}",
                        self.current_score + delta
                    ))
                    .font(theme::subheading_font())
                    .color(theme::GOLD),
                );
                ui.add_space(16.0);
                if ui
                    .button(RichText::new("Return to Menu").color(theme::BLUE_BG))
                    .clicked()
                {
                    action = Some(FinalAction::Done { score_delta: delta });
                }
            }
        });
        action
    }
}
