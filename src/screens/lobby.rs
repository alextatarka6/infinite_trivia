//! Phone lobby — QR code + address on the left, mode / difficulty and the
//! players on the couch on the right. Shown when TEAMS starts with nobody seated.

use egui::{Align2, Key, Pos2, Stroke, Vec2};
use crate::screens::kit::{self, Btn, Canvas, TagStyle};
use crate::server::protocol::PlayerInfo;
use crate::theme::{self, GOLD, INK, LINE, MUTED, PANEL};

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
    /// Shown in place of the join hint when phones can't connect.
    pub warning: Option<String>,
    qr_modules: Option<(usize, Vec<bool>)>,
}

impl LobbyScreen {
    pub fn new(url: &str) -> Self {
        Self {
            mode: LobbyMode::Trivia,
            level: 2,
            enter_start: -1.0,
            url: url.to_string(),
            warning: None,
            qr_modules: kit::qr_modules(url),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, players: &[PlayerInfo]) -> Option<LobbyAction> {
        kit::backdrop(ui.ctx(), None);
        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        if kit::quit_button(ui) {
            return Some(LobbyAction::Back);
        }
        let c = Canvas::new(ui.ctx());
        let painter = ui.painter().clone();

        // -- Top bar ---------------------------------------------------------------
        let tag = kit::tag(&painter, &c, c.p(64.0, 30.0), "LOBBY", TagStyle::Gold, theme::TYPE_TAG);
        kit::label(&painter, &c, Pos2::new(tag.right() + c.v(16.0), tag.center().y), Align2::LEFT_CENTER, "PHONES ARE THE BUZZERS", MUTED);

        // -- Join: big QR + address ---------------------------------------------------
        let qr = c.r(64.0, 110.0, 320.0, 320.0);
        match &self.qr_modules {
            Some(m) => kit::draw_qr(&painter, qr, m),
            None => {
                painter.rect_filled(qr, 0.0, PANEL);
                kit::label(&painter, &c, qr.center(), Align2::CENTER_CENTER, "QR UNAVAILABLE", MUTED);
            }
        }
        kit::label(&painter, &c, c.p(64.0, 466.0), Align2::LEFT_CENTER, "JOIN ON PHONE", MUTED);
        painter.text(c.p(64.0, 500.0), Align2::LEFT_CENTER, self.url.trim_start_matches("http://"), c.cond(800, 38.0), GOLD);
        match &self.warning {
            Some(w) => kit::wrapped(&painter, c.p(64.0, 528.0), w, c.barlow(600, 16.0), theme::BAD, c.v(320.0), c.v(22.0)),
            None => kit::wrapped(&painter, c.p(64.0, 528.0), "Scan the code, or type the address on any phone on this Wi-Fi.",
                c.barlow(500, 16.0), MUTED, c.v(320.0), c.v(22.0)),
        };

        // -- Mode + difficulty --------------------------------------------------------------
        let x = 448.0;
        kit::label(&painter, &c, c.p(x, 122.0), Align2::LEFT_CENTER, "MODE", MUTED);
        let modes = ["TRIVIA", "QUIZBOWL", "JEOPARDY!", "MIX"];
        if let (_, Some(i)) = kit::segmented(ui, &c, c.p(x, 136.0), &modes, self.mode_idx(), "lobby_mode") {
            self.mode = LobbyMode::from_idx(i);
        }
        if !matches!(self.mode, LobbyMode::Jeopardy) {
            kit::label(&painter, &c, c.p(x + 400.0, 122.0), Align2::LEFT_CENTER, "DIFFICULTY", MUTED);
            let levels = ["EASY", "MEDIUM", "HARD", "MIXED"];
            if let (_, Some(i)) = kit::segmented(ui, &c, c.p(x + 400.0, 136.0), &levels, self.level.clamp(1, 4) as usize - 1, "lobby_level") {
                self.level = i as u8 + 1;
            }
        }

        // -- On the couch -------------------------------------------------------------------------
        let n = players.len();
        let count = match n {
            0 => "ON THE COUCH".to_string(),
            1 => "ON THE COUCH · 1 PLAYER".to_string(),
            n => format!("ON THE COUCH · {n} PLAYERS"),
        };
        let lab = kit::label(&painter, &c, c.p(x, 222.0), Align2::LEFT_CENTER, &count, MUTED);
        painter.line_segment([Pos2::new(lab.right() + c.v(16.0), lab.center().y), Pos2::new(c.p(1216.0, 0.0).x, lab.center().y)], Stroke::new(1.0, LINE));
        if n == 0 {
            kit::wrapped(&painter, c.p(x, 250.0), "Nobody yet. Players join from their phones and show up here.",
                c.barlow(500, theme::TYPE_BODY), MUTED, c.v(700.0), c.v(26.0));
        }
        for (i, p) in players.iter().enumerate().take(8) {
            let (col, row) = (i % 2, i / 2);
            let r = c.r(x + col as f32 * 390.0, 244.0 + row as f32 * 84.0, 378.0, 76.0);
            let fade = if p.connected { 1.0 } else { 0.5 };
            kit::poly(&painter, kit::pattern_b(r, c.v(theme::CHAMFER_LG)), PANEL.gamma_multiply(fade), Stroke::NONE);
            let plate = egui::Rect::from_min_size(r.min + Vec2::splat(c.v(10.0)), Vec2::splat(c.v(56.0)));
            kit::photo_plate(&painter, plate, &p.name, kit::team_photo(ui.ctx(), &p.name).as_ref(), false);
            painter.text(Pos2::new(plate.right() + c.v(14.0), r.center().y), Align2::LEFT_CENTER, p.name.to_uppercase(), c.cond(800, 26.0), INK.gamma_multiply(fade));
            kit::label(&painter, &c, Pos2::new(r.right() - c.v(16.0), r.center().y), Align2::RIGHT_CENTER,
                if p.connected { "READY" } else { "AWAY" }, MUTED.gamma_multiply(fade));
        }

        // -- Start ------------------------------------------------------------------------------------
        if n > 0 {
            let br = c.r(x, 600.0, 200.0, 60.0);
            kit::key_hint(&painter, &c, Pos2::new(br.right() + c.v(12.0), br.center().y), Align2::LEFT_CENTER, "ENTER ↵");
            if kit::button(ui, &c, br, "START GAME", Btn::Primary, "lobby_start").clicked()
                || ui.input(|i| i.key_pressed(Key::Enter))
            {
                return Some(match self.mode {
                    LobbyMode::Trivia => LobbyAction::StartTrivia(self.level),
                    LobbyMode::Quizbowl => LobbyAction::StartQuizbowl(self.level),
                    LobbyMode::Jeopardy => LobbyAction::StartJeopardy,
                    LobbyMode::Mix => LobbyAction::StartMix(self.level),
                });
            }
        } else {
            kit::text_tracked(&painter, c.p(x, 630.0), Align2::LEFT_CENTER, "WAITING FOR PLAYERS…", c.cond(800, 28.0), MUTED, 0.06);
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
