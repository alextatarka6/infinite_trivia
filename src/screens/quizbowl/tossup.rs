use egui::RichText;
use crate::api::qbreader::{check_answer, Tossup};
use crate::game::scoring::quizbowl_score;
use crate::theme;

const CHARS_PER_SECOND: f32 = 30.0;

pub enum TossupAction {
    Next,
    Quit,
}

pub enum TossupState {
    Reading,
    BuzzedIn { answer_input: String },
    Result { correct: bool, score_delta: i32 },
    TimedOut,
}

pub struct TossupScreen {
    pub tossup: Tossup,
    pub score: i32,
    pub questions_done: u32,
    pub chars_revealed: usize,
    pub char_timer: f32,
    pub state: TossupState,
    pub loading: bool,
    pub error: Option<String>,
}

impl TossupScreen {
    pub fn new(tossup: Tossup, score: i32, questions_done: u32) -> Self {
        Self {
            tossup,
            score,
            questions_done,
            chars_revealed: 0,
            char_timer: 0.0,
            state: TossupState::Reading,
            loading: false,
            error: None,
        }
    }

    pub fn loading(score: i32, questions_done: u32) -> Self {
        let dummy = Tossup {
            question: String::new(),
            answer: String::new(),
            category: String::new(),
            difficulty: String::new(),
        };
        let mut s = Self::new(dummy, score, questions_done);
        s.loading = true;
        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, dt: f32) -> Option<TossupAction> {
        let mut action = None;

        if self.loading {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(
                    RichText::new("Loading question…")
                        .font(theme::subheading_font())
                        .color(theme::GOLD),
                );
            });
            return None;
        }

        if let Some(err) = &self.error {
            let err = err.clone();
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    RichText::new("Could not fetch question")
                        .font(theme::subheading_font())
                        .color(theme::RED_WRONG),
                );
                ui.label(RichText::new(&err).color(theme::WHITE));
                ui.add_space(16.0);
                if ui.button(RichText::new("Back to Menu").color(theme::BLUE_BG)).clicked() {
                    action = Some(TossupAction::Quit);
                }
            });
            return action;
        }

        let total_chars = self.tossup.question.chars().count();

        // Advance character reveal
        if matches!(self.state, TossupState::Reading) {
            self.char_timer += dt;
            let target = (self.char_timer * CHARS_PER_SECOND) as usize;
            self.chars_revealed = target.min(total_chars);
            if self.chars_revealed >= total_chars {
                self.state = TossupState::TimedOut;
            }
        }

        // Safe UTF-8 slice up to chars_revealed
        let revealed_text: String = self.tossup.question
            .chars()
            .take(self.chars_revealed)
            .collect();

        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "Quizbowl — {} | Difficulty: {}",
                    self.tossup.category, self.tossup.difficulty
                ))
                .font(theme::body_font())
                .color(theme::GOLD),
            );
            ui.label(
                RichText::new(format!(
                    "Q#{} | Score: {}",
                    self.questions_done + 1,
                    self.score
                ))
                .color(theme::WHITE),
            );
            ui.add_space(12.0);

            ui.label(
                RichText::new(&revealed_text)
                    .font(theme::subheading_font())
                    .color(theme::WHITE),
            );
            ui.add_space(16.0);

            match &mut self.state {
                TossupState::Reading => {
                    if ui
                        .button(
                            RichText::new("BUZZ IN [Space]")
                                .font(theme::subheading_font())
                                .color(theme::BLUE_BG),
                        )
                        .clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Space))
                    {
                        self.state = TossupState::BuzzedIn {
                            answer_input: String::new(),
                        };
                    }
                    if ui.button(RichText::new("Quit").color(egui::Color32::LIGHT_GRAY)).clicked() {
                        action = Some(TossupAction::Quit);
                    }
                }

                TossupState::BuzzedIn { answer_input } => {
                    ui.label(RichText::new("Your answer:").color(theme::GOLD));
                    let resp = ui.text_edit_singleline(answer_input);
                    let submitted = resp.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if submitted
                        || ui.button(RichText::new("Submit").color(theme::BLUE_BG)).clicked()
                    {
                        let correct = check_answer(answer_input, &self.tossup.answer);
                        let delta = quizbowl_score(correct, self.chars_revealed, total_chars);
                        self.score += delta;
                        self.state = TossupState::Result { correct, score_delta: delta };
                    }
                }

                TossupState::TimedOut => {
                    ui.label(
                        RichText::new("Time's up!")
                            .font(theme::heading_font())
                            .color(theme::RED_WRONG),
                    );
                    ui.label(
                        RichText::new(format!("Answer: {}", self.tossup.answer))
                            .color(theme::WHITE),
                    );
                    ui.add_space(12.0);
                    if ui.button(RichText::new("Next →").color(theme::BLUE_BG)).clicked() {
                        action = Some(TossupAction::Next);
                    }
                    if ui.button(RichText::new("Quit").color(egui::Color32::LIGHT_GRAY)).clicked() {
                        action = Some(TossupAction::Quit);
                    }
                }

                TossupState::Result { correct, score_delta } => {
                    let (label, color) = if *correct {
                        ("Correct!", theme::GREEN_CORRECT)
                    } else {
                        ("Wrong!", theme::RED_WRONG)
                    };
                    ui.label(RichText::new(label).font(theme::heading_font()).color(color));
                    let delta_str = if *score_delta >= 0 {
                        format!("+{}", score_delta)
                    } else {
                        score_delta.to_string()
                    };
                    ui.label(
                        RichText::new(format!("{} pts | Total: {}", delta_str, self.score))
                            .color(theme::WHITE),
                    );
                    ui.label(
                        RichText::new(format!("Answer: {}", self.tossup.answer))
                            .color(theme::WHITE),
                    );

                    // Show remaining question text after buzz-in point
                    if self.chars_revealed < total_chars {
                        let rest: String = self.tossup.question
                            .chars()
                            .skip(self.chars_revealed)
                            .collect();
                        ui.label(
                            RichText::new(format!("…{}", rest))
                                .font(theme::body_font())
                                .color(egui::Color32::LIGHT_GRAY),
                        );
                    }

                    ui.add_space(12.0);
                    if ui.button(RichText::new("Next →").color(theme::BLUE_BG)).clicked() {
                        action = Some(TossupAction::Next);
                    }
                    if ui.button(RichText::new("Quit").color(egui::Color32::LIGHT_GRAY)).clicked() {
                        action = Some(TossupAction::Quit);
                    }
                }
            }
        });

        action
    }
}
