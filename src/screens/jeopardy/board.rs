use egui::{Color32, FontId, Margin, RichText, Vec2};
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
        let spacing_x = 4.0;
        let col_w = ((ui.available_width() - spacing_x * (n_cols as f32 - 1.0))
            / n_cols as f32)
            .floor()
            .max(100.0);
        let btn_h = 60.0;
        // Fixed header height — large enough for 2-line wrapped category names.
        let header_h = 72.0;
        let pad_h = 8.0;
        let inner_w = col_w - pad_h * 2.0;

        egui::Grid::new("jeopardy_board")
            .num_columns(n_cols)
            .min_col_width(col_w)
            .max_col_width(col_w)
            .spacing(Vec2::new(spacing_x, 4.0))
            .show(ui, |ui| {
                // Category headers: uniform col_w × header_h, text wrapped + centered.
                for (cat, _) in &self.categories {
                    let label = cat.to_uppercase();
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(col_w, header_h),
                        egui::Sense::hover(),
                    );
                    if ui.is_rect_visible(rect) {
                        ui.painter().rect_filled(rect, 6.0, theme::BLUE_DARK);

                        // Layout wrapped, horizontally-centered text.
                        let mut job = egui::text::LayoutJob::default();
                        job.halign = egui::Align::Center;
                        job.wrap.max_width = inner_w;
                        job.append(
                            &label,
                            0.0,
                            egui::text::TextFormat {
                                font_id: FontId::proportional(13.0),
                                color: theme::WHITE,
                                ..Default::default()
                            },
                        );
                        let galley = ui.fonts(|f| f.layout_job(job));
                        let text_h = galley.size().y;
                        // halign=Center means pos.x is the horizontal midpoint of the text.
                        let pos = egui::pos2(
                            rect.center().x,
                            rect.top() + (header_h - text_h) / 2.0,
                        );
                        ui.painter().galley(pos, galley, theme::WHITE);
                    }
                }
                ui.end_row();

                // Value rows: every cell is exactly col_w × btn_h.
                for row in 0..5 {
                    for col in 0..n_cols {
                        let value = jeopardy_value(row, self.is_double_jeopardy);
                        let used = self.used[col][row];
                        if used {
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(col_w, btn_h),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 6.0, theme::BLUE_BG);
                        } else {
                            let btn = egui::Button::new(
                                RichText::new(format!("${}", value))
                                    .font(theme::dollar_font())
                                    .color(theme::GOLD),
                            )
                            .min_size(Vec2::new(col_w, btn_h))
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
