use egui::RichText;
use crate::api::opentdb::TriviaQuestion;
use crate::game::scoring::trivia_score;
use crate::theme;

pub enum TriviaAction {
    Next { score_delta: i32 },
    Quit,
}

pub struct TriviaScreen {
    pub questions: Vec<TriviaQuestion>,
    pub current_idx: usize,
    pub score: i32,
    pub streak: u32,
    pub selected: Option<usize>,
    pub timer: f32,
    pub timer_max: f32,
    pub loading: bool,
    pub error: Option<String>,
}

impl TriviaScreen {
    pub fn new(questions: Vec<TriviaQuestion>) -> Self {
        Self {
            questions,
            current_idx: 0,
            score: 0,
            streak: 0,
            selected: None,
            timer: 20.0,
            timer_max: 20.0,
            loading: false,
            error: None,
        }
    }

    pub fn loading() -> Self {
        let mut s = Self::new(vec![]);
        s.loading = true;
        s
    }

    pub fn with_error(msg: String) -> Self {
        let mut s = Self::new(vec![]);
        s.error = Some(msg);
        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, dt: f32) -> Option<TriviaAction> {
        let mut action = None;

        if self.loading {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(
                    RichText::new("Loading questions…")
                        .font(theme::subheading_font())
                        .color(theme::GOLD),
                );
            });
            return None;
        }

        if let Some(err) = &self.error {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    RichText::new("Could not fetch questions")
                        .font(theme::subheading_font())
                        .color(theme::RED_WRONG),
                );
                ui.label(RichText::new(err).color(theme::WHITE));
                ui.add_space(16.0);
                if ui.button(RichText::new("Back to Menu").color(theme::BLUE_BG)).clicked() {
                    action = Some(TriviaAction::Quit);
                }
            });
            return action;
        }

        if self.current_idx >= self.questions.len() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    RichText::new("Round Complete!")
                        .font(theme::heading_font())
                        .color(theme::GOLD),
                );
                ui.label(
                    RichText::new(format!("Final Score: {}", self.score))
                        .font(theme::subheading_font())
                        .color(theme::WHITE),
                );
                ui.add_space(16.0);
                if ui.button(RichText::new("Back to Menu").color(theme::BLUE_BG)).clicked() {
                    action = Some(TriviaAction::Quit);
                }
            });
            return action;
        }

        let q = &self.questions[self.current_idx];
        let answers = q.shuffled_answers();

        // tick timer only if not yet answered
        if self.selected.is_none() {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.timer = 0.0;
                self.streak = 0;
                self.selected = Some(usize::MAX); // timed out sentinel
            }
        }

        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "Q {}/{} — {}",
                    self.current_idx + 1,
                    self.questions.len(),
                    q.category
                ))
                .font(theme::body_font())
                .color(theme::GOLD),
            );
            ui.label(
                RichText::new(format!("Score: {}", self.score))
                    .color(theme::WHITE),
            );

            // Timer bar
            let ratio = (self.timer / self.timer_max).clamp(0.0, 1.0);
            let bar_color = if ratio > 0.5 {
                theme::GREEN_CORRECT
            } else if ratio > 0.25 {
                theme::GOLD
            } else {
                theme::RED_WRONG
            };
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width() * 0.8, 8.0),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(rect, 4.0, theme::BLUE_MID);
            let mut filled = rect;
            filled.set_right(rect.left() + rect.width() * ratio);
            ui.painter().rect_filled(filled, 4.0, bar_color);

            ui.add_space(16.0);
            ui.label(
                RichText::new(&q.question)
                    .font(theme::subheading_font())
                    .color(theme::WHITE),
            );
            ui.add_space(16.0);

            for (i, (ans, is_correct)) in answers.iter().enumerate() {
                let btn_color = match self.selected {
                    None => theme::BLUE_DARK,
                    Some(sel) if sel == i && *is_correct => theme::GREEN_CORRECT,
                    Some(sel) if sel == i => theme::RED_WRONG,
                    Some(_) if *is_correct => theme::GREEN_CORRECT,
                    _ => theme::BLUE_DARK,
                };
                let btn = egui::Button::new(
                    RichText::new(ans).font(theme::body_font()).color(theme::WHITE),
                )
                .fill(btn_color)
                .min_size(egui::vec2(300.0, 40.0));

                let resp = ui.add(btn);
                if resp.clicked() && self.selected.is_none() {
                    self.selected = Some(i);
                    let delta = trivia_score(*is_correct, self.streak);
                    self.score += delta;
                    if *is_correct {
                        self.streak += 1;
                    } else {
                        self.streak = 0;
                    }
                }
            }

            ui.add_space(16.0);
            if self.selected.is_some() {
                if ui
                    .button(RichText::new("Next →").color(theme::BLUE_BG))
                    .clicked()
                {
                    self.current_idx += 1;
                    self.selected = None;
                    self.timer = self.timer_max;
                    action = Some(TriviaAction::Next { score_delta: 0 });
                }
            }

            ui.add_space(8.0);
            if ui
                .button(RichText::new("Quit").color(egui::Color32::LIGHT_GRAY))
                .clicked()
            {
                action = Some(TriviaAction::Quit);
            }
        });

        action
    }
}
