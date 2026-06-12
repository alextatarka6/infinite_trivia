use egui::{Align2, Color32, Id, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use crate::api::jeopardy::Clue;
use crate::game::scoring::jeopardy_value;
use crate::screens::kit;
use crate::theme::{
    self, ACCENT_SOFT, BLUE_BG, BLUE_DARK, BLUE_MID, GOLD, GOLD_HOVER, INK_DIM, LINE, ON_ACCENT,
    TILE_BOTTOM, WHITE,
};

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
    enter_start: f64,
}

/// `$1,200` / `-$400`
pub fn money(n: i32) -> String {
    let mag = n.unsigned_abs();
    let digits: Vec<char> = mag.to_string().chars().collect();
    let mut out = String::new();
    for (i, c) in digits.iter().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*c);
    }
    if n < 0 { format!("-${out}") } else { format!("${out}") }
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
            enter_start: -1.0,
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

    /// Pixel width of a `.it-team` chip for the given name + score.
    fn chip_width(painter: &egui::Painter, name: &str, score: i32) -> f32 {
        let name_w = painter.layout_no_wrap(name.into(), theme::mono(9.5), WHITE).size().x;
        let score_w = painter.layout_no_wrap(money(score), theme::mono(14.0), WHITE).size().x;
        7.0 + 20.0 + 9.0 + name_w + 9.0 + score_w + 13.0
    }

    /// One `.it-team` chip, drawn with its left edge at `left`.
    fn team_chip(
        painter: &egui::Painter,
        ui: &egui::Ui,
        left: f32,
        cy: f32,
        idx: usize,
        name: &str,
        score: i32,
        active: bool,
    ) {
        let no = (idx + 1).to_string();
        let score_s = money(score);
        let name_font = theme::mono(9.5);
        let score_font = theme::mono(14.0);
        let circle_d = 20.0;
        let gap = 9.0;
        let pad_l = 7.0;
        let pad_r = 13.0;
        let w = Self::chip_width(painter, name, score);
        let h = 34.0;
        let rect = Rect::from_min_size(Pos2::new(left, cy - h / 2.0), Vec2::new(w, h));

        let lift = if active { -2.0 } else { 0.0 };
        let r = rect.translate(Vec2::new(0.0, lift));
        let fill = if active {
            kit::lerp_color(BLUE_MID, GOLD, 0.10)
        } else {
            BLUE_MID
        };
        painter.rect_filled(r, Rounding::same(999.0), fill);
        painter.rect_stroke(
            r,
            Rounding::same(999.0),
            Stroke::new(1.0, if active { GOLD } else { LINE }),
        );
        if active {
            painter.rect_stroke(r, Rounding::same(999.0), Stroke::new(1.0, ACCENT_SOFT));
        }

        // number circle
        let cc = Pos2::new(r.left() + pad_l + circle_d / 2.0, r.center().y);
        painter.circle_filled(cc, circle_d / 2.0, if active { GOLD } else { BLUE_BG });
        if !active {
            painter.circle_stroke(cc, circle_d / 2.0, Stroke::new(1.0, LINE));
        }
        painter.text(
            cc,
            Align2::CENTER_CENTER,
            no,
            theme::mono(10.0),
            if active { ON_ACCENT } else { INK_DIM },
        );
        // name
        let nx = cc.x + circle_d / 2.0 + gap;
        painter.text(
            Pos2::new(nx, r.center().y),
            Align2::LEFT_CENTER,
            name,
            name_font,
            if active { GOLD } else { INK_DIM },
        );
        // score
        painter.text(
            Pos2::new(r.right() - pad_r, r.center().y),
            Align2::RIGHT_CENTER,
            score_s,
            score_font,
            WHITE,
        );
        let _ = ui;
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<BoardAction> {
        kit::ambient(ui.ctx());
        let mut action = None;
        if kit::quit_button(ui) {
            action = Some(BoardAction::Quit);
        }

        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        let ey = kit::enter_offset(ui, self.enter_start);

        let screen = ui.ctx().screen_rect();
        let avail_w = ui.available_width();
        let painter = ui.painter().clone();
        let has_teams = self.players.len() > 1;

        // -- Round title (.it-round) ------------------------------------------
        let mut y = 24.0 + ey;
        let title = if self.is_double_jeopardy { "DOUBLE JEOPARDY!" } else { "JEOPARDY!" };
        painter.text(
            Pos2::new(avail_w / 2.0, y + 18.0),
            Align2::CENTER_CENTER,
            title,
            theme::spectral(30.0),
            GOLD,
        );
        y += 44.0;

        // -- Scoreboard (.it-teams) -------------------------------------------
        let chip_gap = 9.0;
        let names: Vec<String> = self.players.iter().map(|(n, _)| n.chars().take(10).collect()).collect();
        let widths: Vec<f32> = self
            .players
            .iter()
            .zip(&names)
            .map(|((_, s), n)| Self::chip_width(&painter, n, *s))
            .collect();
        let total_w: f32 = widths.iter().sum::<f32>() + chip_gap * (widths.len() as f32 - 1.0);
        let mut cx = (avail_w - total_w) / 2.0;
        for (i, ((_, score), name)) in self.players.iter().zip(&names).enumerate() {
            Self::team_chip(&painter, ui, cx, y + 17.0, i, name, *score, i == self.active);
            cx += widths[i] + chip_gap;
        }
        y += 34.0 + 10.0;

        // turn line (.it-turn)
        if has_teams {
            painter.text(
                Pos2::new(avail_w / 2.0, y),
                Align2::CENTER_CENTER,
                format!("{} has the board", self.players[self.active].0),
                theme::mono(11.0),
                INK_DIM,
            );
            y += 22.0;
        }

        // -- Board grid (manual, flexes to fill) ------------------------------
        let n_cols = self.categories.len().max(1);
        let side_pad = 30.0;
        let gap = 7.0;
        let grid_left = side_pad;
        let grid_w = avail_w - side_pad * 2.0;
        let col_w = (grid_w - gap * (n_cols as f32 - 1.0)) / n_cols as f32;
        let header_h = 64.0;

        let final_btn = self.all_used();
        let bottom_reserve = if final_btn { 78.0 } else { 28.0 };
        let grid_top = y + 4.0;
        let grid_bottom = screen.bottom() - bottom_reserve + ey;
        let rows = 5usize;
        let tile_h = (grid_bottom - grid_top - header_h - gap * rows as f32) / rows as f32;

        for col in 0..n_cols {
            let x = grid_left + col as f32 * (col_w + gap);

            // category header (.it-cat)
            let hrect = Rect::from_min_size(Pos2::new(x, grid_top), Vec2::new(col_w, header_h));
            painter.rect_filled(hrect, Rounding::same(10.0), BLUE_MID);
            painter.rect_stroke(hrect, Rounding::same(10.0), Stroke::new(1.0, LINE));
            let cat = self.categories[col].0.to_uppercase();
            let galley = painter.layout(cat, theme::plex_semibold(11.0), WHITE, col_w - 16.0);
            painter.galley(
                Pos2::new(hrect.center().x - galley.size().x / 2.0, hrect.center().y - galley.size().y / 2.0),
                galley,
                WHITE,
            );

            // value tiles (.it-tile)
            for row in 0..rows {
                let ty = grid_top + header_h + gap + row as f32 * (tile_h + gap);
                let rect = Rect::from_min_size(Pos2::new(x, ty), Vec2::new(col_w, tile_h));
                let used = self.used[col][row];
                let resp = ui.interact(
                    rect,
                    Id::new(("jd_tile", col, row)),
                    if used { Sense::hover() } else { Sense::click() },
                );

                if used {
                    painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0, LINE));
                    continue;
                }

                let t = ui.ctx().animate_bool(Id::new(("jd_tile_h", col, row)), resp.hovered());
                let lift = -4.0 * t;
                let grow = 1.0 + 0.04 * t;
                let draw = Rect::from_center_size(
                    rect.center() + Vec2::new(0.0, lift),
                    rect.size() * grow,
                );
                if t > 0.0 {
                    painter.rect_filled(
                        draw.expand(6.0 * t),
                        Rounding::same(10.0),
                        ACCENT_SOFT.gamma_multiply(0.5 * t),
                    );
                }
                // gradient-ish fill: base tile, darker bottom inset
                painter.rect_filled(draw, Rounding::same(10.0), BLUE_DARK);
                let bottom = Rect::from_min_max(
                    Pos2::new(draw.left(), draw.center().y),
                    draw.max,
                );
                painter.rect_filled(bottom, Rounding::same(10.0), TILE_BOTTOM.gamma_multiply(0.5));
                // inset top highlight
                painter.rect_filled(
                    Rect::from_min_size(draw.min + Vec2::new(8.0, 1.0), Vec2::new(draw.width() - 16.0, 1.0)),
                    Rounding::ZERO,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 25),
                );
                painter.text(
                    draw.center(),
                    Align2::CENTER_CENTER,
                    money(jeopardy_value(row, self.is_double_jeopardy)),
                    theme::mono_semibold(26.0),
                    kit::lerp_color(GOLD, GOLD_HOVER, t),
                );

                if resp.clicked() {
                    action = Some(BoardAction::SelectClue { col, row });
                }
            }
        }

        // -- Final Jeopardy button --------------------------------------------
        if final_btn
            && kit::centered_primary(ui, avail_w / 2.0, screen.bottom() - 28.0 - 42.0 + ey, "Final Jeopardy →")
        {
            action = Some(BoardAction::GoToFinal);
        }

        action
    }
}
