use egui::{RichText, Stroke};
use crate::api::jeopardy::{check_answer, Clue};
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
        let avail = ui.available_size();
        let top = ((avail.y - 400.0) / 2.0).max(20.0);
        ui.add_space(top);

        ui.vertical_centered(|ui| {
            ui.set_max_width(640.0);

            // -- Title --------------------------------------------------------
            ui.label(
                RichText::new("FINAL JEOPARDY!")
                    .font(egui::FontId::proportional(40.0))
                    .color(GOLD)
                    .strong(),
            );
            ui.add_space(10.0);

            // -- Category card ------------------------------------------------
            ui.label(
                RichText::new("CATEGORY")
                    .font(theme::mono_font(10.0))
                    .color(INK_DIM),
            );
            ui.add_space(6.0);

            egui::Frame::none()
                .fill(BLUE_MID)
                .rounding(egui::Rounding::same(12.0))
                .stroke(Stroke::new(1.0, LINE))
                .inner_margin(egui::Margin::symmetric(32.0, 16.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(&self.clue.category)
                            .font(egui::FontId::proportional(30.0))
                            .color(WHITE),
                    );
                });

            ui.add_space(22.0);

            if self.show_category {
                // Wager phase
                ui.label(
                    RichText::new(format!(
                        "SCORE  ${}  |  MAX WAGER  ${}",
                        self.current_score,
                        self.current_score.max(0)
                    ))
                    .font(theme::mono_font(11.0))
                    .color(INK_DIM),
                );
                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    ui.add_space((avail.x - 300.0).max(0.0) / 2.0);
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
                            let max = self.current_score.max(0);
                            self.wager = Some(w.clamp(0, max));
                            self.show_category = false;
                        }
                    }
                });
                ui.add_space(16.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Lock wager >")
                                .color(egui::Color32::from_rgb(26, 19, 4))
                                .font(theme::body_font()),
                        )
                        .fill(GOLD)
                        .min_size(egui::vec2(148.0, 38.0)),
                    )
                    .clicked()
                {
                    if let Ok(w) = self.wager_input.trim().parse::<i32>() {
                        let max = self.current_score.max(0);
                        self.wager = Some(w.clamp(0, max));
                        self.show_category = false;
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
                        self.answered = Some(check_answer(&self.answer_input, &self.clue.answer));
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
                    self.answered = Some(check_answer(&self.answer_input, &self.clue.answer));
                }
            } else {
                // Result phase
                let correct = self.answered.unwrap();
                let wager = self.wager.unwrap_or(0);
                let delta = if correct { wager } else { -wager };

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
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("Final score: ${}", self.current_score + delta))
                        .font(egui::FontId::proportional(20.0))
                        .color(GOLD),
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
                            RichText::new("Return to menu")
                                .color(egui::Color32::from_rgb(26, 19, 4))
                                .font(theme::body_font()),
                        )
                        .fill(GOLD)
                        .min_size(egui::vec2(160.0, 38.0)),
                    )
                    .clicked()
                {
                    action = Some(FinalAction::Done { score_delta: delta });
                }
            }
        });

        action
    }
}
