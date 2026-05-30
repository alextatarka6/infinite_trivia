use egui::{Color32, FontId, RichText, Vec2};
use crate::api::jeopardy::Clue;
use crate::game::scoring::jeopardy_value;
use crate::theme;

pub enum BoardAction {
    SelectClue { col: usize, row: usize },
    GoToFinal,
    Quit,
}

pub struct Board {
    pub categories: Vec<(String, Vec<Clue>)>,
    pub used: [[bool; 5]; 6],
    pub is_double_jeopardy: bool,
    pub score: i32,
}

impl Board {
    pub fn new(categories: Vec<(String, Vec<Clue>)>, is_double_jeopardy: bool) -> Self {
        Self {
            categories,
            used: [[false; 5]; 6],
            is_double_jeopardy,
            score: 0,
        }
    }

    pub fn all_used(&self) -> bool {
        self.used.iter().all(|col| col.iter().all(|&u| u))
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<BoardAction> {
        let mut action = None;

        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            let round_label = if self.is_double_jeopardy {
                "DOUBLE JEOPARDY!"
            } else {
                "JEOPARDY!"
            };
            ui.label(
                RichText::new(round_label)
                    .font(theme::heading_font())
                    .color(theme::GOLD),
            );
            ui.label(
                RichText::new(format!("Score: ${}", self.score))
                    .font(theme::subheading_font())
                    .color(theme::WHITE),
            );
            ui.add_space(8.0);
        });

        let n_cols = self.categories.len();
        egui::Grid::new("jeopardy_board")
            .num_columns(n_cols)
            .spacing(Vec2::new(4.0, 4.0))
            .show(ui, |ui| {
                // Category headers
                for (cat, _) in &self.categories {
                    let label = cat.to_uppercase();
                    ui.vertical_centered(|ui| {
                        ui.set_min_size(Vec2::new(120.0, 60.0));
                        ui.painter().rect_filled(
                            ui.available_rect_before_wrap(),
                            6.0,
                            theme::BLUE_DARK,
                        );
                        ui.label(
                            RichText::new(&label)
                                .font(FontId::proportional(13.0))
                                .color(theme::WHITE)
                                .strong(),
                        );
                    });
                }
                ui.end_row();

                // Value rows
                for row in 0..5 {
                    for col in 0..n_cols {
                        let value = jeopardy_value(row, self.is_double_jeopardy);
                        let used = self.used[col][row];
                        if used {
                            ui.add_space(120.0);
                        } else {
                            let btn = egui::Button::new(
                                RichText::new(format!("${}", value))
                                    .font(theme::dollar_font())
                                    .color(theme::GOLD),
                            )
                            .min_size(Vec2::new(120.0, 60.0))
                            .fill(theme::BLUE_DARK);

                            if ui.add(btn).clicked() {
                                action = Some(BoardAction::SelectClue { col, row });
                            }
                        }
                    }
                    ui.end_row();
                }
            });

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if self.all_used() {
                if ui
                    .button(
                        RichText::new("Final Jeopardy →")
                            .color(theme::GOLD)
                            .font(theme::subheading_font()),
                    )
                    .clicked()
                {
                    action = Some(BoardAction::GoToFinal);
                }
            }
            if ui
                .button(RichText::new("Quit").color(Color32::LIGHT_GRAY))
                .clicked()
            {
                action = Some(BoardAction::Quit);
            }
        });

        action
    }
}
