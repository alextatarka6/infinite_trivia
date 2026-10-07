use egui::{Align2, Color32, Id, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use crate::screens::kit;
use crate::theme::{self, BLUE_DARK, BLUE_MID, GOLD, INK_DIM, LINE, WHITE};
use crate::server::protocol::PlayerInfo;

pub enum LobbyMode {
    Trivia,
    Quizbowl,
    Jeopardy,
    Mix,
}

pub enum LobbyAction {
    StartTrivia(u8),
    StartQuizbowl(u8),
    StartJeopardy,
    StartMix(u8),
    Back,
}

pub struct LobbyScreen {
    pub mode: LobbyMode,
    pub level: u8,
    enter_start: f64,
    url: String,
    qr_modules: Option<(usize, Vec<bool>)>,
}

impl LobbyScreen {
    pub fn new(url: &str) -> Self {
        let qr_modules = qrcode::QrCode::new(url.as_bytes()).ok().map(|code| {
            let w = code.width();
            let mut dark = Vec::with_capacity(w * w);
            for r in 0..w {
                for c in 0..w {
                    dark.push(code[(r, c)] == qrcode::Color::Dark);
                }
            }
            (w, dark)
        });
        Self {
            mode: LobbyMode::Trivia,
            level: 2,
            enter_start: -1.0,
            url: url.to_string(),
            qr_modules,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, players: &[PlayerInfo]) -> Option<LobbyAction> {
        kit::ambient(ui.ctx());
        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        let ey = kit::enter_offset(ui, self.enter_start);

        if kit::quit_button(ui) {
            return Some(LobbyAction::Back);
        }

        let avail = ui.available_size();
        let cx = avail.x / 2.0;
        let painter = ui.painter().clone();

        let mut y = 36.0 + ey;

        // Header
        painter.text(
            Pos2::new(cx, y),
            Align2::CENTER_CENTER,
            "MULTIPLAYER",
            theme::spectral_medium(15.0),
            INK_DIM,
        );
        y += 34.0;

        // QR code
        let qr_size = 148.0;
        if let Some((w, dark)) = &self.qr_modules {
            let module_px = qr_size / *w as f32;
            let tl = Pos2::new(cx - qr_size / 2.0 - 6.0, y);
            let bg = Rect::from_min_size(tl, Vec2::splat(qr_size + 12.0));
            painter.rect_filled(bg, Rounding::same(6.0), Color32::WHITE);
            for row in 0..*w {
                for col in 0..*w {
                    if dark[row * w + col] {
                        let px = tl.x + 6.0 + col as f32 * module_px;
                        let py = tl.y + 6.0 + row as f32 * module_px;
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(px, py), Vec2::splat(module_px + 0.5)),
                            Rounding::ZERO,
                            Color32::BLACK,
                        );
                    }
                }
            }
        } else {
            // Fallback box when QR generation fails
            let box_r = Rect::from_center_size(Pos2::new(cx, y + qr_size / 2.0), Vec2::splat(qr_size));
            painter.rect_filled(box_r, Rounding::same(8.0), BLUE_DARK);
            painter.rect_stroke(box_r, Rounding::same(8.0), Stroke::new(1.0, LINE));
            painter.text(box_r.center(), Align2::CENTER_CENTER, "QR unavailable", theme::plex(12.0), INK_DIM);
        }
        y += qr_size + 18.0;

        // URL
        painter.text(
            Pos2::new(cx, y),
            Align2::CENTER_CENTER,
            &self.url,
            theme::mono_semibold(13.0),
            GOLD,
        );
        y += 26.0;

        painter.text(
            Pos2::new(cx, y),
            Align2::CENTER_CENTER,
            "Scan QR or type URL on your phone",
            theme::plex(11.5),
            INK_DIM,
        );
        y += 28.0;

        // Divider
        painter.line_segment(
            [Pos2::new(cx - 80.0, y), Pos2::new(cx + 80.0, y)],
            Stroke::new(1.0, LINE),
        );
        y += 16.0;

        // Player list
        let n = players.len();
        let count_label = if n == 0 {
            "No players yet".to_string()
        } else {
            format!("{} player{} connected", n, if n == 1 { "" } else { "s" })
        };
        painter.text(
            Pos2::new(cx, y),
            Align2::CENTER_CENTER,
            &count_label,
            theme::plex_medium(12.5),
            if n == 0 { INK_DIM } else { WHITE },
        );
        y += 22.0;

        let chip_h = 34.0;
        let chip_w = (avail.x * 0.55).min(240.0);
        for (i, player) in players.iter().enumerate() {
            let rect = Rect::from_center_size(
                Pos2::new(cx, y + chip_h / 2.0),
                Vec2::new(chip_w, chip_h),
            );
            painter.rect_filled(rect, Rounding::same(8.0), BLUE_MID);
            painter.rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0, LINE));
            let dot_c = Pos2::new(rect.left() + 14.0, rect.center().y);
            painter.circle_filled(dot_c, 4.0, GOLD.gamma_multiply(0.6 + 0.4 * ((i as f32 * 0.7 + now as f32 * 0.9).sin() * 0.5 + 0.5)));
            painter.text(
                Pos2::new(dot_c.x + 12.0, rect.center().y),
                Align2::LEFT_CENTER,
                &player.name,
                theme::plex_medium(13.0),
                WHITE,
            );
            y += chip_h + 6.0;
        }

        y += 10.0;

        // Divider
        painter.line_segment(
            [Pos2::new(cx - 80.0, y), Pos2::new(cx + 80.0, y)],
            Stroke::new(1.0, LINE),
        );
        y += 16.0;

        // Mode tabs
        painter.text(
            Pos2::new(cx, y),
            Align2::CENTER_CENTER,
            "GAME MODE",
            theme::mono(9.5),
            INK_DIM,
        );
        y += 16.0;

        let tab_labels = ["Trivia", "Quizbowl", "Jeopardy", "Mix"];
        let tab_w = 76.0;
        let tab_h = 30.0;
        let tab_gap = 5.0;
        let tabs_total = tab_labels.len() as f32 * tab_w + (tab_labels.len() - 1) as f32 * tab_gap;
        let tab_x0 = cx - tabs_total / 2.0;
        let mut new_mode: Option<usize> = None;

        for (i, label) in tab_labels.iter().enumerate() {
            let rect = Rect::from_min_size(
                Pos2::new(tab_x0 + i as f32 * (tab_w + tab_gap), y),
                Vec2::new(tab_w, tab_h),
            );
            let selected = self.mode_idx() == i;
            let resp = ui.interact(rect, Id::new(("lobby_tab", i)), Sense::click());
            painter.rect_filled(
                rect,
                Rounding::same(6.0),
                if selected { GOLD } else { BLUE_MID },
            );
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                theme::plex_medium(11.5),
                if selected { Color32::from_rgb(18, 13, 0) } else { INK_DIM },
            );
            if resp.clicked() {
                new_mode = Some(i);
            }
        }

        if let Some(i) = new_mode {
            self.mode = LobbyMode::from_idx(i);
        }
        y += tab_h + 14.0;

        // Difficulty dots (not for Jeopardy)
        if !matches!(self.mode, LobbyMode::Jeopardy) {
            painter.text(
                Pos2::new(cx, y),
                Align2::CENTER_CENTER,
                "DIFFICULTY",
                theme::mono(9.5),
                INK_DIM,
            );
            y += 16.0;

            let dot_r = 7.0;
            let dot_spacing = 36.0;
            let level_names = ["Easy", "Medium", "Hard"];
            let dots_total = 3.0 * dot_spacing;
            let dot_x0 = cx - dots_total / 2.0 + dot_spacing / 2.0;

            for n in 1u8..=3 {
                let dc = Pos2::new(dot_x0 + (n as f32 - 1.0) * dot_spacing, y + dot_r);
                let r = if n == self.level { dot_r + 2.0 } else { dot_r };
                let fill = if n <= self.level { GOLD } else { BLUE_MID };
                let hit = Rect::from_center_size(dc, Vec2::splat(r * 2.5));
                let resp = ui.interact(hit, Id::new(("lobby_diff", n)), Sense::click());
                painter.circle_filled(dc, r, fill);
                if n == self.level {
                    painter.circle_stroke(dc, r + 3.0, Stroke::new(1.0, GOLD.gamma_multiply(0.4)));
                }
                painter.text(
                    Pos2::new(dc.x, dc.y + dot_r + 9.0),
                    Align2::CENTER_CENTER,
                    level_names[n as usize - 1],
                    theme::mono(8.5),
                    INK_DIM,
                );
                if resp.clicked() {
                    self.level = n;
                }
            }
            y += dot_r * 2.0 + 24.0;
        } else {
            y += 8.0;
        }

        // Start / waiting
        let can_start = n > 0;
        if can_start {
            let btn_rect = Rect::from_center_size(
                Pos2::new(cx, y + 21.0),
                Vec2::new(180.0, 42.0),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(btn_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            if kit::primary_button(&mut child, "Start Game →").clicked() {
                return Some(match self.mode {
                    LobbyMode::Trivia => LobbyAction::StartTrivia(self.level),
                    LobbyMode::Quizbowl => LobbyAction::StartQuizbowl(self.level),
                    LobbyMode::Jeopardy => LobbyAction::StartJeopardy,
                    LobbyMode::Mix => LobbyAction::StartMix(self.level),
                });
            }
        } else {
            painter.text(
                Pos2::new(cx, y + 21.0),
                Align2::CENTER_CENTER,
                "Waiting for players to join…",
                theme::plex_medium(12.5),
                INK_DIM,
            );
        }

        None
    }

    fn mode_idx(&self) -> usize {
        match self.mode {
            LobbyMode::Trivia => 0,
            LobbyMode::Quizbowl => 1,
            LobbyMode::Jeopardy => 2,
            LobbyMode::Mix => 3,
        }
    }
}

impl LobbyMode {
    pub fn from_idx(i: usize) -> Self {
        match i {
            0 => LobbyMode::Trivia,
            1 => LobbyMode::Quizbowl,
            2 => LobbyMode::Jeopardy,
            _ => LobbyMode::Mix,
        }
    }
}
