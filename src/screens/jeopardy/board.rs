//! Jeopardy board — "02 Board" in the Roster hi-fi.

use egui::{Align2, Id, Key, Pos2, Rect, Sense, Stroke, Vec2};
use crate::api::jeopardy::Clue;
use crate::game::scoring::jeopardy_value;
use crate::screens::kit::{self, Btn, Canvas, RosterEntry, TagStyle, Texture};
use crate::theme::{self, BG, CHAMFER_MD, GOLD, INK, LINE, MUTED, ON_GOLD, PANEL, PANEL_HI};

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
    /// Daily Doubles not yet found (set by App each frame; it owns their positions).
    pub dd_left: usize,
    /// Keyboard/hover focus (col, row).
    focus: Option<(usize, usize)>,
    enter_start: f64,
}

/// `$1,200` / `−$400`
pub fn money(n: i32) -> String {
    kit::money(n)
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
            dd_left: 0,
            focus: None,
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

    fn clues_left(&self) -> usize {
        let rows = self.categories.first().map(|(_, v)| v.len()).unwrap_or(5).min(5);
        (0..self.categories.len().min(6))
            .map(|c| (0..rows).filter(|&r| !self.used[c][r]).count())
            .sum()
    }

    /// Arrow keys move the focus over live tiles; Enter/Space picks it.
    fn keyboard(&mut self, ui: &egui::Ui, n_cols: usize, rows: usize) -> Option<(usize, usize)> {
        let (dx, dy, pick) = ui.input(|i| {
            let dx = i.key_pressed(Key::ArrowRight) as i32 - i.key_pressed(Key::ArrowLeft) as i32;
            let dy = i.key_pressed(Key::ArrowDown) as i32 - i.key_pressed(Key::ArrowUp) as i32;
            (dx, dy, i.key_pressed(Key::Enter) || i.key_pressed(Key::Space))
        });
        if dx != 0 || dy != 0 {
            let (c, r) = self.focus.unwrap_or((0, 0));
            let c = (c as i32 + dx).clamp(0, n_cols as i32 - 1) as usize;
            let r = (r as i32 + dy).clamp(0, rows as i32 - 1) as usize;
            self.focus = Some((c, r));
        }
        match self.focus {
            Some((c, r)) if pick && !self.used[c][r] => Some((c, r)),
            _ => None,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<BoardAction> {
        kit::backdrop(ui.ctx(), None);
        let c = Canvas::new(ui.ctx());
        let painter = ui.painter().clone();
        let mut action = None;
        if kit::quit_button(ui) {
            action = Some(BoardAction::Quit);
        }
        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        let age = kit::since(ui, self.enter_start, 1.2);

        // -- Top bar --------------------------------------------------------------
        let round = if self.is_double_jeopardy { "ROUND 2" } else { "ROUND 1" };
        let t = kit::tag(&painter, &c, c.p(40.0, 28.0), round, TagStyle::Gold, theme::TYPE_TAG);
        let title = if self.is_double_jeopardy { "DOUBLE JEOPARDY!" } else { "JEOPARDY!" };
        kit::text_tracked(&painter, Pos2::new(t.right() + c.v(16.0), t.center().y), Align2::LEFT_CENTER, title,
            c.cond(700, 16.0), MUTED, 0.14);
        let left = self.clues_left();
        let status = match self.dd_left {
            0 => format!("{left} CLUES LEFT"),
            1 => format!("{left} CLUES LEFT · DAILY DOUBLE STILL HIDDEN"),
            n => format!("{left} CLUES LEFT · {n} DAILY DOUBLES HIDDEN"),
        };
        kit::text_tracked(&painter, c.p(1131.0, 46.0), Align2::RIGHT_CENTER, &status, c.cond(700, 15.0), MUTED, 0.12);

        // -- Grid ---------------------------------------------------------------------
        let n_cols = self.categories.len().clamp(1, 6);
        let rows = 5usize;
        let gap = theme::SPACE_GAP;
        let (gx, gw) = (40.0, 1200.0);
        let col_w = (gw - gap * (n_cols as f32 - 1.0)) / n_cols as f32;
        let (head_top, head_h, tile_top, tile_h) = (82.0, 72.0, 160.0, 84.0);
        if let Some((col, row)) = self.keyboard(ui, n_cols, rows) {
            action = Some(BoardAction::SelectClue { col, row });
        }

        for col in 0..n_cols {
            let x = gx + col as f32 * (col_w + gap);
            // Category header: panel with a 3px gold underline.
            let hr = c.r(x, head_top, col_w, head_h);
            painter.rect_filled(hr, 0.0, PANEL);
            painter.rect_filled(c.r(x, head_top + head_h - 3.0, col_w, 3.0), 0.0, GOLD);
            let cat = self.categories[col].0.to_uppercase();
            let max_w = hr.width() - c.v(16.0);
            let size = kit::fit_size(&painter, &cat, 700, c.v(theme::TYPE_CATEGORY), c.v(10.0), max_w, c.v(head_h - 14.0), 1.15);
            let mut job = egui::text::LayoutJob::default();
            job.halign = egui::Align::Center;
            job.wrap.max_width = max_w;
            job.append(&cat, 0.0, egui::TextFormat { font_id: theme::cond(700, size), color: INK,
                line_height: Some(size * 1.15), extra_letter_spacing: size * 0.04, ..Default::default() });
            let galley = painter.layout_job(job);
            let gy = hr.center().y - c.v(1.5) - galley.size().y / 2.0;
            painter.galley(Pos2::new(hr.center().x, gy), galley, INK);

            for row in 0..rows {
                let rect = c.r(x, tile_top + row as f32 * (tile_h + gap), col_w, tile_h);
                let used = self.used[col][row];
                // Diagonal fill-in from the top-left as the board appears.
                let delay = (col + row) as f32 * 0.05;
                let appear = kit::ease_out((age - delay) / 0.22);
                if appear <= 0.0 {
                    continue;
                }
                let rect = rect.translate(Vec2::new(0.0, c.v(10.0) * (1.0 - appear)));
                if used {
                    painter.rect_filled(rect, 0.0, BG);
                    let pts = vec![rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()];
                    kit::texture(&painter, &pts, Texture::Hatch, c.v(12.0), theme::alpha(MUTED, 0.06 * appear));
                    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, LINE.gamma_multiply(appear)));
                    continue;
                }
                let resp = ui.interact(rect, Id::new(("jd_tile", col, row)), Sense::click());
                if resp.hovered() {
                    self.focus = Some((col, row));
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let on = ui.ctx().animate_bool_with_time(Id::new(("jd_tile_f", col, row)), self.focus == Some((col, row)), 0.1);
                let fill = kit::lerp_color(PANEL_HI, GOLD, on).gamma_multiply(appear);
                kit::poly(&painter, kit::pattern_a(rect, c.v(CHAMFER_MD)), fill, Stroke::NONE);
                painter.text(rect.center(), Align2::CENTER_CENTER, money(jeopardy_value(row, self.is_double_jeopardy)),
                    c.cond(800, theme::TYPE_TILE_VALUE), kit::lerp_color(GOLD, ON_GOLD, on).gamma_multiply(appear));
                if resp.clicked() {
                    action = Some(BoardAction::SelectClue { col, row });
                }
            }
        }

        // -- Round over: on to the next round -------------------------------------------
        if self.all_used() {
            let label = if self.is_double_jeopardy { "FINAL JEOPARDY" } else { "DOUBLE JEOPARDY" };
            let size = kit::button_size(&painter, &c, label, theme::TYPE_BUTTON, Vec2::new(28.0, 16.0));
            let br = Rect::from_center_size(c.p(640.0, 380.0), size);
            if kit::button(ui, &c, br, label, Btn::Primary, "board_next").clicked()
                || ui.input(|i| i.key_pressed(Key::Enter))
            {
                action = Some(BoardAction::GoToFinal);
            }
            kit::key_hint(&painter, &c, Pos2::new(br.right() + c.v(12.0), br.center().y), Align2::LEFT_CENTER, "ENTER ↵");
        }

        // -- Roster -----------------------------------------------------------------------
        let picking = self.players.len() > 1;
        let entries: Vec<RosterEntry> = self
            .players
            .iter()
            .enumerate()
            .map(|(i, (name, score))| {
                let mut e = RosterEntry::new(name, *score);
                e.money = true;
                if picking && i == self.active {
                    e.note = Some("PICKING".into());
                }
                e
            })
            .collect();
        let w = (entries.len() as f32 * 400.0).min(1200.0);
        kit::roster(ui, &c, 40.0, w, 772.0, &entries, Some(self.active), false);
        action
    }
}
