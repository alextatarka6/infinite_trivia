use egui::{RichText, Stroke};
use crate::api::jeopardy::{check_answer, Clue};
use crate::theme::{self, BLUE_MID, GOLD, GREEN_CORRECT, INK_DIM, LINE, RED_WRONG, WHITE};

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
}

pub enum ClueAction {
    Done { correct: bool },
}

impl ClueScreen {
    pub fn new(clue: Clue) -> Self {
        Self { clue, input: String::new(), state: ClueState::default() }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<ClueAction> {
        let mut action = None;

        let avail = ui.available_size();
        let top = ((avail.y - 360.0) / 2.0).max(20.0);
        ui.add_space(top);

        ui.vertical_centered(|ui| {
            ui.set_max_width(720.0);

            // -- Category + value meta ----------------------------------------
            ui.horizontal(|ui| {
                ui.add_space((avail.x - 300.0).max(0.0) / 2.0);
                ui.label(
                    RichText::new(self.clue.category.to_uppercase())
                        .font(theme::mono_font(12.0))
                        .color(GOLD),
                );
                ui.label(
                    RichText::new("|")
                        .font(theme::mono_font(12.0))
                        .color(INK_DIM),
                );
                ui.label(
                    RichText::new(format!("${}", self.clue.value))
                        .font(theme::mono_font(12.0))
                        .color(INK_DIM),
                );
            });

            ui.add_space(20.0);

            // -- Question -----------------------------------------------------
            ui.label(
                RichText::new(&self.clue.question)
                    .font(egui::FontId::proportional(28.0))
                    .color(WHITE),
            );

            ui.add_space(28.0);

            match &self.state {
                ClueState::ShowQuestion => {
                    // "What is" prompt + input
                    ui.horizontal(|ui| {
                        ui.add_space((avail.x - 420.0).max(0.0) / 2.0);
                        ui.label(
                            RichText::new("What is")
                                .font(theme::mono_font(12.0))
                                .color(GOLD),
                        );
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.input)
                                .desired_width(300.0)
                                .font(theme::mono_font(16.0))
                                .text_color(WHITE),
                        );
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let correct = check_answer(&self.input, &self.clue.answer);
                            self.state = ClueState::Answered { correct };
                        }
                    });

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        ui.add_space((avail.x - 200.0).max(0.0) / 2.0);
                        let submit = ui.add(
                            egui::Button::new(
                                RichText::new("Submit")
                                    .color(egui::Color32::from_rgb(26, 19, 4))
                                    .font(theme::body_font()),
                            )
                            .fill(GOLD)
                            .min_size(egui::vec2(100.0, 38.0)),
                        );
                        let reveal = ui.add(
                            egui::Button::new(
                                RichText::new("Reveal").color(INK_DIM).font(theme::body_font()),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(Stroke::new(1.0, LINE))
                            .min_size(egui::vec2(88.0, 38.0)),
                        );
                        if submit.clicked() {
                            let correct = check_answer(&self.input, &self.clue.answer);
                            self.state = ClueState::Answered { correct };
                        }
                        if reveal.clicked() {
                            self.state = ClueState::Answered { correct: false };
                        }
                    });
                }

                ClueState::Answered { correct } => {
                    let c = *correct;

                    // Verdict label
                    let (verdict_text, verdict_color) = if c {
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

                    if c {
                        ui.label(
                            RichText::new(format!("+${}", self.clue.value))
                                .font(egui::FontId::proportional(22.0))
                                .color(GREEN_CORRECT),
                        );
                    }

                    ui.add_space(12.0);

                    // Answer reveal card
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(480.0);
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
                        action = Some(ClueAction::Done { correct: c });
                    }
                }
            }
        });

        action
    }
}
