use egui::RichText;
use crate::api::jeopardy::{check_answer, Clue};
use crate::theme;

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

        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(
                RichText::new(self.clue.category.to_uppercase())
                    .font(theme::subheading_font())
                    .color(theme::GOLD),
            );
            ui.label(
                RichText::new(format!("${}", self.clue.value))
                    .font(theme::heading_font())
                    .color(theme::WHITE),
            );
            ui.add_space(20.0);

            ui.label(
                RichText::new(&self.clue.question)
                    .font(theme::subheading_font())
                    .color(theme::WHITE),
            );
            ui.add_space(20.0);

            match &self.state {
                ClueState::ShowQuestion => {
                    ui.label(
                        RichText::new("What is…?")
                            .font(theme::body_font())
                            .color(theme::GOLD),
                    );
                    let resp = ui.text_edit_singleline(&mut self.input);
                    ui.add_space(8.0);
                    let submitted = resp.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if submitted
                        || ui
                            .button(RichText::new("Submit").color(theme::BLUE_BG))
                            .clicked()
                    {
                        let correct = check_answer(&self.input, &self.clue.answer);
                        self.state = ClueState::Answered { correct };
                    }
                }
                ClueState::Answered { correct } => {
                    let (label, color) = if *correct {
                        ("Correct!", theme::GREEN_CORRECT)
                    } else {
                        ("Incorrect", theme::RED_WRONG)
                    };
                    ui.label(RichText::new(label).font(theme::heading_font()).color(color));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Answer: {}", self.clue.answer))
                            .font(theme::body_font())
                            .color(theme::WHITE),
                    );
                    ui.add_space(16.0);
                    let c = *correct;
                    if ui
                        .button(RichText::new("Back to Board").color(theme::BLUE_BG))
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
