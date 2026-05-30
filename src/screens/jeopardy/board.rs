use egui::{Align2, FontId, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use crate::api::jeopardy::Clue;
use crate::game::scoring::jeopardy_value;
use crate::theme::{self, BLUE_BG, BLUE_DARK, BLUE_MID, GOLD, INK_DIM, LINE, WHITE};

pub enum BoardAction {
    SelectClue { col: usize, row: usize },
    GoToFinal,
    Quit,
}

pub struct Board {
    pub categories: Vec<(String, Vec<Clue>)>,
    pub used: [[bool; 5]; 6],
    pub is_double_jeopardy: bool,
    pub players: Vec<(String, i32)>,
    pub active: usize,
}

impl Board {
    pub fn new(
        categories: Vec<(String, Vec<Clue>)>,
        is_double_jeopardy: bool,
        players: Vec<(String, i32)>,
    ) -> Self {
        Self {
            categories,
            used: [[false; 5]; 6],
            is_double_jeopardy,
            players,
            active: 0,
        }
    }

    pub fn active_score(&self) -> i32 {
        self.players[self.active].1
    }

    pub fn advance_turn(&mut self) {
        if self.players.len() > 1 {
            self.active = (self.active + 1) % self.players.len();
        }
    }

    pub fn all_used(&self) -> bool {
        self.used.iter().all(|col| col.iter().all(|&u| u))
    }

    fn draw_score_chip(painter: &egui::Painter, right_x: f32, center_y: f32, score: i32) {
        let label = "SCORE";
        let value = if score < 0 {
            format!("-${}", score.unsigned_abs().to_string()
                .chars().rev().collect::<Vec<_>>()
                .chunks(3).map(|c| c.iter().collect::<String>())
                .collect::<Vec<_>>().join(",")
                .chars().rev().collect::<String>())
        } else {
            format!("${}", score)
        };
        let chip_w = 130.0_f32;
        let chip_h = 28.0_f32;
        let chip_rect = Rect::from_center_size(
            Pos2::new(right_x - chip_w / 2.0 - 8.0, center_y),
            Vec2::new(chip_w, chip_h),
        );
        painter.rect_filled(chip_rect, Rounding::same(999.0), BLUE_MID);
        painter.rect_stroke(chip_rect, Rounding::same(999.0), Stroke::new(1.0, LINE));

        let mid = chip_rect.center();
        painter.text(
            Pos2::new(mid.x - 4.0, mid.y),
            Align2::RIGHT_CENTER,
            label,
            theme::mono_font(10.0),
            INK_DIM,
        );
        painter.text(
            Pos2::new(mid.x, mid.y),
            Align2::LEFT_CENTER,
            format!("  {}", value),
            FontId::proportional(13.0),
            WHITE,
        );
    }

    fn draw_player_chips(
        painter: &egui::Painter,
        players: &[(String, i32)],
        active: usize,
        center_y: f32,
        avail_w: f32,
    ) {
        let chip_w = 110.0_f32;
        let chip_h = 26.0_f32;
        let chip_gap = 8.0_f32;
        let n = players.len() as f32;
        let total_w = n * chip_w + (n - 1.0) * chip_gap;
        let start_x = (avail_w - total_w) / 2.0;

        for (i, (name, score)) in players.iter().enumerate() {
            let is_active = i == active;
            let x = start_x + i as f32 * (chip_w + chip_gap);
            let chip_rect = Rect::from_min_size(
                Pos2::new(x, center_y - chip_h / 2.0),
                Vec2::new(chip_w, chip_h),
            );
            painter.rect_filled(chip_rect, Rounding::same(999.0), BLUE_MID);
            let border_color = if is_active { GOLD } else { LINE };
            let border_w = if is_active { 2.0 } else { 1.0 };
            painter.rect_stroke(chip_rect, Rounding::same(999.0), Stroke::new(border_w, border_color));

            let mid = chip_rect.center();
            let name_color = if is_active { GOLD } else { INK_DIM };
            let display_name: String = name.chars().take(8).collect();
            painter.text(
                Pos2::new(mid.x - 4.0, mid.y),
                Align2::RIGHT_CENTER,
                display_name,
                theme::mono_font(9.0),
                name_color,
            );
            let score_str = if *score < 0 {
                format!(" -${}", score.unsigned_abs())
            } else {
                format!(" ${}", score)
            };
            painter.text(
                Pos2::new(mid.x - 4.0, mid.y),
                Align2::LEFT_CENTER,
                score_str,
                FontId::proportional(12.0),
                WHITE,
            );
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<BoardAction> {
        let mut action = None;

        let has_teams = self.players.len() > 1;
        let bar_h = if has_teams { 80.0_f32 } else { 46.0_f32 };
        let avail_w = ui.available_width();
        let (bar_rect, _) =
            ui.allocate_exact_size(Vec2::new(avail_w, bar_h), Sense::hover());

        let round_label = if self.is_double_jeopardy {
            "DOUBLE JEOPARDY!"
        } else {
            "JEOPARDY!"
        };
        let label_y = if has_teams { bar_rect.top() + 22.0 } else { bar_rect.center().y };
        ui.painter().text(
            Pos2::new(bar_rect.center().x, label_y),
            Align2::CENTER_CENTER,
            round_label,
            theme::heading_font(),
            GOLD,
        );

        if has_teams {
            Self::draw_player_chips(
                ui.painter(),
                &self.players,
                self.active,
                bar_rect.top() + 58.0,
                avail_w,
            );
        } else {
            Self::draw_score_chip(
                ui.painter(),
                bar_rect.right(),
                bar_rect.center().y,
                self.players[0].1,
            );
        }

        ui.add_space(6.0);

        // ── Grid ─────────────────────────────────────────────────────────────
        let n_cols = self.categories.len();
        let spacing = 5.0_f32;
        let col_w = ((ui.available_width() - spacing * (n_cols as f32 - 1.0))
            / n_cols as f32)
            .floor()
            .max(80.0);
        let header_h = 68.0_f32;
        let tile_h = 62.0_f32;
        let inner_w = col_w - 12.0;

        egui::Grid::new("jeopardy_board")
            .num_columns(n_cols)
            .min_col_width(col_w)
            .max_col_width(col_w)
            .spacing(Vec2::new(spacing, spacing))
            .show(ui, |ui| {
                // Category headers
                for (cat, _) in &self.categories {
                    let label = cat.to_uppercase();
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(col_w, header_h), Sense::hover());
                    if ui.is_rect_visible(rect) {
                        ui.painter().rect_filled(rect, Rounding::same(8.0), BLUE_MID);
                        ui.painter().rect_stroke(
                            rect,
                            Rounding::same(8.0),
                            Stroke::new(1.0, LINE),
                        );
                        let mut job = egui::text::LayoutJob::default();
                        job.halign = egui::Align::Center;
                        job.wrap.max_width = inner_w;
                        job.append(
                            &label,
                            0.0,
                            egui::text::TextFormat {
                                font_id: FontId::proportional(11.5),
                                color: WHITE,
                                ..Default::default()
                            },
                        );
                        let galley = ui.fonts(|f| f.layout_job(job));
                        let text_h = galley.size().y;
                        let pos =
                            Pos2::new(rect.center().x, rect.top() + (header_h - text_h) / 2.0);
                        ui.painter().galley(pos, galley, WHITE);
                    }
                }
                ui.end_row();

                // Value rows
                for row in 0..5 {
                    for col in 0..n_cols {
                        let value = jeopardy_value(row, self.is_double_jeopardy);
                        let used = self.used[col][row];

                        let (rect, resp) = ui.allocate_exact_size(
                            Vec2::new(col_w, tile_h),
                            if used { Sense::hover() } else { Sense::click() },
                        );
                        if ui.is_rect_visible(rect) {
                            if used {
                                ui.painter().rect_stroke(
                                    rect,
                                    Rounding::same(8.0),
                                    Stroke::new(1.0, LINE),
                                );
                            } else {
                                let fill = if resp.hovered() {
                                    egui::Color32::from_rgb(34, 60, 170)
                                } else {
                                    BLUE_DARK
                                };
                                ui.painter().rect_filled(rect, Rounding::same(8.0), fill);
                                let top_line = Rect::from_min_size(
                                    rect.min + Vec2::new(8.0, 1.0),
                                    Vec2::new(col_w - 16.0, 1.0),
                                );
                                ui.painter().rect_filled(
                                    top_line,
                                    Rounding::ZERO,
                                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25),
                                );
                                ui.painter().text(
                                    rect.center(),
                                    Align2::CENTER_CENTER,
                                    format!("${}", value),
                                    theme::dollar_font(),
                                    GOLD,
                                );
                                if resp.clicked() {
                                    action = Some(BoardAction::SelectClue { col, row });
                                }
                            }
                        }
                    }
                    ui.end_row();
                }
            });

        ui.add_space(14.0);

        // ── Bottom actions ────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            if self.all_used() {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Final Jeopardy >")
                                .font(theme::subheading_font())
                                .color(egui::Color32::from_rgb(26, 19, 4)),
                        )
                        .fill(GOLD),
                    )
                    .clicked()
                {
                    action = Some(BoardAction::GoToFinal);
                }
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("Quit").color(INK_DIM))
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0, LINE)),
                )
                .clicked()
            {
                action = Some(BoardAction::Quit);
            }
        });

        let _ = BLUE_BG;
        action
    }
}
