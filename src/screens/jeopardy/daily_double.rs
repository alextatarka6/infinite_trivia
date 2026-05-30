use egui::{RichText, Stroke};
use crate::api::jeopardy::{check_answer, Clue};
use crate::theme::{self, BLUE_MID, GOLD, GREEN_CORRECT, INK_DIM, LINE, RED_WRONG, WHITE};

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
        let avail = ui.available_size();
        let top = ((avail.y - 380.0) / 2.0).max(20.0);
        ui.add_space(top);

        ui.vertical_centered(|ui| {
            ui.set_max_width(640.0);

            // Splash
            ui.label(
                RichText::new("DAILY DOUBLE!")
                    .font(egui::FontId::proportional(52.0))
                    .color(GOLD)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(self.clue.category.to_uppercase())
                    .font(theme::mono_font(12.0))
                    .color(GOLD),
            );
            ui.add_space(20.0);

            if self.wager.is_none() {
                // Wager phase
                let max_wager = self.current_score.max(1000);
                ui.label(
                    RichText::new(format!(
                        "SCORE  ${}  |  MAX WAGER  ${}",
                        self.current_score, max_wager
                    ))
                    .font(theme::mono_font(11.0))
                    .color(INK_DIM),
                );
                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    ui.add_space((avail.x - 320.0).max(0.0) / 2.0);
                    ui.label(
                        RichText::new("WAGER  $")
                            .font(theme::mono_font(11.0))
                            .color(INK_DIM),
                    );
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.wager_input)
                            .desired_width(160.0)
                            .font(theme::mono_font(16.0))
                            .text_color(WHITE),
                    );
                    let submitted =
                        resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if submitted {
                        if let Ok(w) = self.wager_input.trim().parse::<i32>() {
                            self.wager = Some(w.clamp(5, max_wager));
                        }
                    }
                });
                ui.add_space(16.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Confirm wager >")
                                .color(egui::Color32::from_rgb(26, 19, 4))
                                .font(theme::body_font()),
                        )
                        .fill(GOLD)
                        .min_size(egui::vec2(160.0, 38.0)),
                    )
                    .clicked()
                {
                    if let Ok(w) = self.wager_input.trim().parse::<i32>() {
                        let max_wager = self.current_score.max(1000);
                        self.wager = Some(w.clamp(5, max_wager));
                    }
                }
            } else if self.answered.is_none() {
                // Question phase
                ui.label(
                    RichText::new(&self.clue.question)
                        .font(egui::FontId::proportional(26.0))
                        .color(WHITE),
                );
                ui.add_space(24.0);

                ui.horizontal(|ui| {
                    ui.add_space((avail.x - 400.0).max(0.0) / 2.0);
                    ui.label(
                        RichText::new("What is")
                            .font(theme::mono_font(12.0))
                            .color(GOLD),
                    );
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.answer_input)
                            .desired_width(280.0)
                            .font(theme::mono_font(16.0))
                            .text_color(WHITE),
                    );
                    let submitted =
                        resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if submitted {
                        let correct = check_answer(&self.answer_input, &self.clue.answer);
                        self.answered = Some(correct);
                    }
                });
                ui.add_space(16.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Submit")
                                .color(egui::Color32::from_rgb(26, 19, 4))
                                .font(theme::body_font()),
                        )
                        .fill(GOLD)
                        .min_size(egui::vec2(100.0, 38.0)),
                    )
                    .clicked()
                {
                    let correct = check_answer(&self.answer_input, &self.clue.answer);
                    self.answered = Some(correct);
                }
            } else {
                // Result phase
                let correct = self.answered.unwrap();
                let wager = self.wager.unwrap();
                let winnings = if correct { wager } else { -wager };

                let (verdict_text, verdict_color) = if correct {
                    ("CORRECT", GREEN_CORRECT)
                } else {
                    ("INCORRECT", RED_WRONG)
                };
                ui.label(
                    RichText::new(verdict_text)
                        .font(egui::FontId::proportional(44.0))
                        .color(verdict_color)
                        .strong(),
                );
                ui.label(
                    RichText::new(format!(
                        "{}${}",
                        if correct { "+" } else { "-" },
                        wager
                    ))
                    .font(egui::FontId::proportional(22.0))
                    .color(verdict_color),
                );
                ui.add_space(12.0);

                egui::Frame::none()
                    .fill(BLUE_MID)
                    .rounding(egui::Rounding::same(12.0))
                    .stroke(Stroke::new(1.0, LINE))
                    .inner_margin(egui::Margin::symmetric(20.0, 12.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("CORRECT RESPONSE")
                                .font(theme::mono_font(10.0))
                                .color(INK_DIM),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&self.clue.answer)
                                .font(egui::FontId::proportional(20.0))
                                .color(WHITE),
                        );
                    });

                ui.add_space(20.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Back to board >")
                                .color(egui::Color32::from_rgb(26, 19, 4))
                                .font(theme::body_font()),
                        )
                        .fill(GOLD)
                        .min_size(egui::vec2(160.0, 38.0)),
                    )
                    .clicked()
                {
                    action = Some(DailyDoubleAction::Done { winnings });
                }
            }
        });

        action
    }
}
