use egui::{Align2, FontId, Pos2, Rect, RichText, Rounding, Sense, Shape, Stroke, Vec2};
use crate::game::state::Screen;
use crate::theme::{
    ACCENT_SOFT, BLUE_MID, GOLD, INK_DIM, LINE, WHITE,
};

pub struct HomeScreen {
    pub level: u8, // 1=Easy 2=Medium 3=Hard
}

impl Default for HomeScreen {
    fn default() -> Self {
        Self { level: 2 }
    }
}

impl HomeScreen {
    // -- Glyph painters -------------------------------------------------------

    fn draw_board_glyph(painter: &egui::Painter, center: Pos2) {
        let sq = 5.5_f32;
        let step = sq + 2.5;
        let offset = step; // distance from center to far edge's start
        for row in 0..3i32 {
            for col in 0..3i32 {
                let rect = Rect::from_min_size(
                    Pos2::new(
                        center.x - offset + col as f32 * step,
                        center.y - offset + row as f32 * step,
                    ),
                    Vec2::splat(sq),
                );
                painter.rect_filled(rect, Rounding::same(1.5), GOLD);
            }
        }
    }

    fn draw_target_glyph(painter: &egui::Painter, center: Pos2) {
        painter.circle_stroke(center, 11.0, Stroke::new(2.5, GOLD));
        painter.circle_filled(center, 4.0, GOLD);
    }

    fn draw_play_glyph(painter: &egui::Painter, center: Pos2) {
        let pts = vec![
            Pos2::new(center.x - 8.0, center.y - 11.0),
            Pos2::new(center.x - 8.0, center.y + 11.0),
            Pos2::new(center.x + 10.0, center.y),
        ];
        painter.add(Shape::convex_polygon(pts, GOLD, Stroke::NONE));
    }

    fn draw_card(
        painter: &egui::Painter,
        rect: Rect,
        hovered: bool,
        glyph: &str,
        name: &str,
    ) {
        painter.rect_filled(rect, Rounding::same(14.0), BLUE_MID);
        let border = if hovered { GOLD } else { LINE };
        painter.rect_stroke(rect, Rounding::same(14.0), Stroke::new(1.0, border));

        let glyph_center = Pos2::new(rect.center().x, rect.top() + 60.0);
        match glyph {
            "board" => Self::draw_board_glyph(painter, glyph_center),
            "target" => Self::draw_target_glyph(painter, glyph_center),
            _ => Self::draw_play_glyph(painter, glyph_center),
        }

        let name_color = if hovered { GOLD } else { WHITE };
        painter.text(
            Pos2::new(rect.center().x, rect.top() + 102.0),
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(22.0),
            name_color,
        );
    }

    // -- Score / info chip ----------------------------------------------------
    pub fn score_chip(painter: &egui::Painter, rect: Rect, label: &str, value: &str) {
        painter.rect_filled(rect, Rounding::same(999.0), BLUE_MID);
        painter.rect_stroke(rect, Rounding::same(999.0), Stroke::new(1.0, LINE));
        let mid = rect.center();
        // Label on left of center
        painter.text(
            Pos2::new(mid.x - 6.0, mid.y),
            Align2::RIGHT_CENTER,
            label,
            crate::theme::mono_font(10.5),
            INK_DIM,
        );
        // Value on right of center
        painter.text(
            Pos2::new(mid.x - 2.0, mid.y),
            Align2::LEFT_CENTER,
            value,
            FontId::proportional(14.0),
            WHITE,
        );
    }

    // -- Main show ------------------------------------------------------------
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<(Screen, u8)> {
        let mut result: Option<(Screen, u8)> = None;
        let mut new_level: Option<u8> = None;
        let current_level = self.level;

        let avail = ui.available_size();
        let content_h = 440.0_f32;
        let top_pad = ((avail.y - content_h) / 2.0).max(16.0);
        ui.add_space(top_pad);

        ui.vertical_centered(|ui| {
            // -- Wordmark -----------------------------------------------------
            ui.label(
                RichText::new("INFINITE")
                    .font(FontId::proportional(20.0))
                    .color(WHITE.linear_multiply(0.8)),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new("TRIVIA")
                    .font(FontId::proportional(70.0))
                    .color(GOLD)
                    .strong(),
            );

            ui.add_space(36.0);

            // -- Mode cards ---------------------------------------------------
            let card_w = 200.0_f32;
            let card_h = 155.0_f32;
            let gap = 18.0_f32;

            ui.horizontal(|ui| {
                let total_cards_w = 3.0 * card_w + 2.0 * gap;
                ui.add_space((avail.x - total_cards_w) / 2.0);

                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::click());
                Self::draw_card(ui.painter(), rect, resp.hovered(), "board", "Jeopardy!");
                if resp.clicked() {
                    result = Some((Screen::JeopardySetup, current_level));
                }

                ui.add_space(gap);

                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::click());
                Self::draw_card(ui.painter(), rect, resp.hovered(), "target", "General Trivia");
                if resp.clicked() {
                    result = Some((Screen::Trivia, current_level));
                }

                ui.add_space(gap);

                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::click());
                Self::draw_card(ui.painter(), rect, resp.hovered(), "play", "Quizbowl");
                if resp.clicked() {
                    result = Some((Screen::Quizbowl, current_level));
                }
            });

            ui.add_space(28.0);

            // -- Difficulty label ---------------------------------------------
            let level_names = ["EASY", "MEDIUM", "HARD"];
            ui.label(
                RichText::new(format!(
                    "DIFFICULTY  |  {}",
                    level_names[current_level as usize - 1]
                ))
                .font(crate::theme::mono_font(11.0))
                .color(INK_DIM),
            );

            ui.add_space(10.0);

            // -- Difficulty dots ----------------------------------------------
            let dot_r = 7.0_f32;
            let dot_r_sel = dot_r + 2.5;
            let dot_slot = dot_r_sel * 2.0; // uniform allocation for all dots
            let rail_gap = 60.0_f32;
            let row_w = 3.0 * dot_slot + 2.0 * rail_gap;
            let left_margin = ((avail.x - row_w) / 2.0) - 8.0; // -1 to account for stroke width on first dot

            ui.horizontal(|ui| {
                ui.add_space(left_margin.max(0.0));

                for n in 1u8..=3 {
                    let r = if n == current_level { dot_r_sel } else { dot_r };
                    let (dot_rect, dot_resp) =
                        ui.allocate_exact_size(Vec2::splat(dot_slot), Sense::click());
                    let c = dot_rect.center();

                    let fill = if n <= current_level { GOLD } else { BLUE_MID };
                    ui.painter().circle_filled(c, r, fill);
                    if n == current_level {
                        ui.painter().circle_stroke(
                            c,
                            r + 3.0,
                            Stroke::new(1.0, ACCENT_SOFT),
                        );
                    }
                    let border_c = if n <= current_level { GOLD } else { LINE };
                    ui.painter().circle_stroke(c, r, Stroke::new(1.5, border_c));

                    if dot_resp.clicked() {
                        new_level = Some(n);
                    }

                    if n < 3 {
                        // rail connector
                        let (rail_rect, _) =
                            ui.allocate_exact_size(Vec2::new(rail_gap, 4.0), Sense::hover());
                        let y = rail_rect.center().y;
                        let filled_frac = if current_level > n { 1.0 } else { 0.0 };
                        // background rail
                        ui.painter().line_segment(
                            [
                                Pos2::new(rail_rect.min.x, y),
                                Pos2::new(rail_rect.max.x, y),
                            ],
                            Stroke::new(2.0, LINE),
                        );
                        // filled portion
                        if filled_frac > 0.0 {
                            ui.painter().line_segment(
                                [
                                    Pos2::new(rail_rect.min.x, y),
                                    Pos2::new(
                                        rail_rect.min.x + rail_rect.width() * filled_frac,
                                        y,
                                    ),
                                ],
                                Stroke::new(2.0, GOLD),
                            );
                        }
                    }
                }
            });

            ui.add_space(22.0);

            ui.label(
                RichText::new("pick a mode to start")
                    .font(crate::theme::mono_font(11.0))
                    .color(INK_DIM),
            );
        });

        if let Some(lv) = new_level {
            self.level = lv;
        }

        result
    }
}
