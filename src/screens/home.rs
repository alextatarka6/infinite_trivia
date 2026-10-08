//! Home — "01 Home" in the Roster hi-fi. Logo top-left, difficulty + players
//! segments top-right, four slanted mode cards (the selected one wide), and the
//! "ON THE COUCH" roster with the join-on-phone card.

use egui::{Align2, Id, Pos2, Rect, Sense, Stroke, Vec2};

use crate::screens::kit::{self, Btn, Canvas, Texture};
use crate::screens::lobby::LobbyMode;
use crate::server::protocol::PlayerInfo;
use crate::theme::{self, GOLD, INK, LINE, MUTED, PANEL, SLANT_CARD};

pub enum HomeAction {
    /// Start `mode` at `level` (1 easy … 4 mixed). `solo` = no phones.
    Start { mode: LobbyMode, level: u8, solo: bool },
    /// The player picked TEAMS: start the phone server so the QR code shows.
    OpenTeams,
}

/// What the couch row shows: the phone server's URL and seated players.
pub struct Couch<'a> {
    pub url: Option<&'a str>,
    pub players: &'a [PlayerInfo],
}

struct Mode {
    name: &'static str,
    /// Line breaks for the narrow (unselected) card.
    narrow: &'static [&'static str],
    blurb: &'static str,
    short: &'static str,
    texture: Texture,
    mode: fn() -> LobbyMode,
}

const MODES: [Mode; 4] = [
    Mode {
        name: "JEOPARDY!",
        narrow: &["JEOPARDY!"],
        blurb: "Six categories, thirty clues, one Daily Double. Wrong answers cost you.",
        short: "Pick a clue, wager big",
        texture: Texture::Grid,
        mode: || LobbyMode::Jeopardy,
    },
    Mode {
        name: "GENERAL TRIVIA",
        narrow: &["GENERAL", "TRIVIA"],
        blurb: "Ten questions, four choices, a clock. Streaks multiply your score.",
        short: "10 questions, 4 choices",
        texture: Texture::Hatch,
        mode: || LobbyMode::Trivia,
    },
    Mode {
        name: "QUIZBOWL",
        narrow: &["QUIZ", "BOWL"],
        blurb: "Tossups read out word by word. Buzz early for power, then play the bonus.",
        short: "Buzz before the end",
        texture: Texture::Scan,
        mode: || LobbyMode::Quizbowl,
    },
    Mode {
        name: "MIX",
        narrow: &["MIX"],
        blurb: "A round of each: Jeopardy, trivia and quizbowl, back to back.",
        short: "All three, shuffled",
        texture: Texture::Checker,
        mode: || LobbyMode::Mix,
    },
];

pub struct HomeScreen {
    /// 1 easy, 2 medium, 3 hard, 4 mixed.
    pub level: u8,
    pub solo: bool,
    pub selected: usize,
    qr: Option<(String, (usize, Vec<bool>))>,
}

impl Default for HomeScreen {
    fn default() -> Self {
        Self { level: 2, solo: true, selected: 0, qr: None }
    }
}

impl HomeScreen {
    pub fn show(&mut self, ui: &mut egui::Ui, couch: Couch) -> Option<HomeAction> {
        kit::backdrop(ui.ctx(), None);
        let c = Canvas::new(ui.ctx());
        let painter = ui.painter().clone();
        let mut action = None;

        // Keyboard: ←/→ choose a mode, Enter starts it.
        let typing = ui.ctx().wants_keyboard_input();
        if !typing {
            ui.input(|i| {
                if i.key_pressed(egui::Key::ArrowRight) {
                    self.selected = (self.selected + 1) % MODES.len();
                }
                if i.key_pressed(egui::Key::ArrowLeft) {
                    self.selected = (self.selected + MODES.len() - 1) % MODES.len();
                }
            });
        }
        let mut start = !typing && ui.input(|i| i.key_pressed(egui::Key::Enter));

        // -- Logo ---------------------------------------------------------------
        let icon_rect = c.r(48.0, 40.0, 48.0, 48.0);
        if let Some(icon) = kit::app_icon(ui.ctx()) {
            painter.image(icon.id(), icon_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), egui::Color32::WHITE);
        }
        let wm = kit::text_tracked(&painter, c.p(112.0, 64.0), Align2::LEFT_CENTER, "INFINITE ", c.cond(800, theme::TYPE_WORDMARK), INK, 0.06);
        kit::text_tracked(&painter, Pos2::new(wm.right(), wm.center().y), Align2::LEFT_CENTER, "TRIVIA", c.cond(800, theme::TYPE_WORDMARK), GOLD, 0.06);

        // -- Difficulty + players (right-aligned) ----------------------------------
        let players_labels = ["SOLO", "TEAMS"];
        let diff_labels = ["EASY", "MEDIUM", "HARD", "MIXED"];
        let pw = kit::segmented_size(&painter, &c, &players_labels).x;
        let dw = kit::segmented_size(&painter, &c, &diff_labels).x;
        let right = c.p(1232.0, 0.0).x;
        let seg_y = c.p(0.0, 62.0).y;
        let px = right - pw;
        let dx = px - c.v(28.0) - dw;
        kit::label(&painter, &c, Pos2::new(dx, c.p(0.0, 50.0).y), Align2::LEFT_CENTER, "DIFFICULTY", MUTED);
        kit::label(&painter, &c, Pos2::new(px, c.p(0.0, 50.0).y), Align2::LEFT_CENTER, "PLAYERS", MUTED);
        if let (_, Some(i)) = kit::segmented(ui, &c, Pos2::new(dx, seg_y), &diff_labels, self.level as usize - 1, "home_diff") {
            self.level = i as u8 + 1;
        }
        if let (_, Some(i)) = kit::segmented(ui, &c, Pos2::new(px, seg_y), &players_labels, usize::from(!self.solo), "home_players") {
            self.solo = i == 0;
            if !self.solo && couch.url.is_none() {
                action = Some(HomeAction::OpenTeams);
            }
        }

        // -- Mode cards ---------------------------------------------------------------
        let gap = 10.0;
        let (x0, x1, top, h) = (48.0, 1232.0, 136.0, 400.0);
        let wide = 600.0;
        let narrow = (x1 - x0 - wide - gap * 3.0) / 3.0;
        let ts: Vec<f32> = (0..MODES.len())
            .map(|i| ui.ctx().animate_bool_with_time(Id::new(("home_mode", i)), self.selected == i, 0.25))
            .collect();
        let mut x = x0;
        for (i, m) in MODES.iter().enumerate() {
            let t = ts[i];
            let w = narrow + (wide - narrow) * t;
            let rect = c.r(x, top, w, h);
            x += w + gap;
            let pts = kit::slant_pts(rect, c.v(SLANT_CARD));
            let resp = ui.interact(rect, Id::new(("home_card", i)), Sense::click());
            let hover = ui.ctx().animate_bool_with_time(resp.id, resp.hovered() && t < 0.5, 0.12);
            kit::poly(&painter, pts.clone(), kit::lerp_color(PANEL, theme::PANEL_HI, hover * 0.6), Stroke::NONE);
            kit::texture(&painter, &pts, m.texture, c.v(m.texture.pitch()), theme::alpha(MUTED, m.texture.alpha()));
            if t > 0.01 {
                kit::inner_stroke(&painter, &pts, c.v(3.5), GOLD.gamma_multiply(t));
            }
            if resp.clicked() {
                if self.selected == i {
                    start = true;
                } else {
                    self.selected = i;
                }
            }
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            let clip = painter.with_clip_rect(rect.shrink2(Vec2::new(c.v(SLANT_CARD) * 0.5, 0.0)).intersect(painter.clip_rect()));
            // Narrow content fades out as the card widens; wide content fades in.
            if t < 0.99 {
                let a = 1.0 - t;
                let lx = rect.left() + c.v(44.0);
                let avail = rect.width() - c.v(44.0 + SLANT_CARD + 12.0);
                // Stack upward from the description: name lines, then the number.
                let widest = m.narrow.iter().map(|l| kit::text_width(&clip, l, c.cond(800, theme::TYPE_MODE_NAME), 0.0)).fold(0.0, f32::max);
                let size = if widest > avail { theme::TYPE_MODE_NAME * avail / widest } else { theme::TYPE_MODE_NAME };
                let lh = c.v(size * 0.88);
                let name_top = c.p(0.0, top + 318.0).y - lh * m.narrow.len() as f32;
                kit::label(&clip, &c, Pos2::new(lx, name_top - c.v(14.0)), Align2::LEFT_CENTER, &format!("{:02}", i + 1), MUTED.gamma_multiply(a));
                for (k, line) in m.narrow.iter().enumerate() {
                    clip.text(Pos2::new(lx, name_top + lh * k as f32), Align2::LEFT_TOP, *line, c.cond(800, size), INK.gamma_multiply(a));
                }
                kit::wrapped(&clip, Pos2::new(lx, c.p(0.0, top + 334.0).y), m.short, c.barlow(500, 15.0), MUTED.gamma_multiply(a), avail.max(c.v(60.0)), c.v(18.0));
            }
            if t > 0.01 {
                let a = t;
                let lx = rect.left() + c.v(60.0);
                kit::label(&clip, &c, Pos2::new(lx, c.p(0.0, top + 118.0).y), Align2::LEFT_CENTER, &format!("{:02} · SELECTED", i + 1), GOLD.gamma_multiply(a));
                let avail = rect.width() - c.v(60.0 + SLANT_CARD + 40.0);
                let nw = kit::text_width(&clip, m.name, c.cond(800, theme::TYPE_MODE_HERO), 0.0);
                let size = if nw > avail { theme::TYPE_MODE_HERO * avail / nw } else { theme::TYPE_MODE_HERO };
                clip.text(Pos2::new(lx - c.v(4.0), c.p(0.0, top + 236.0).y), Align2::LEFT_BOTTOM, m.name, c.cond(800, size), INK.gamma_multiply(a));
                kit::wrapped(&clip, Pos2::new(lx, c.p(0.0, top + 238.0).y), m.blurb, c.barlow(500, theme::TYPE_BODY), MUTED.gamma_multiply(a), c.v(410.0), c.v(27.0));
                if t > 0.9 {
                    let br = Rect::from_min_size(Pos2::new(lx, c.p(0.0, top + 315.0).y), Vec2::new(c.v(158.0), c.v(48.0)));
                    if kit::button(ui, &c, br, "START GAME", Btn::Primary, ("home_start", i)).clicked() {
                        start = true;
                    }
                    kit::key_hint(&painter, &c, Pos2::new(br.right() + c.v(12.0), br.center().y), Align2::LEFT_CENTER, "ENTER ↵");
                }
            }
        }

        // -- On the couch -----------------------------------------------------------------
        let lab = kit::label(&painter, &c, c.p(48.0, 581.0), Align2::LEFT_CENTER, "ON THE COUCH", MUTED);
        painter.line_segment([Pos2::new(lab.right() + c.v(16.0), lab.center().y), Pos2::new(c.p(1232.0, 0.0).x, lab.center().y)], Stroke::new(1.0, LINE));
        if let Some(a) = self.couch(ui, &c, &couch) {
            action = Some(a);
        }

        if start {
            action = Some(HomeAction::Start { mode: (MODES[self.selected].mode)(), level: self.level, solo: self.solo });
        }
        action
    }

    /// The roster row: one card per seated phone (or YOU when solo), then the join card.
    fn couch(&mut self, ui: &mut egui::Ui, c: &Canvas, couch: &Couch) -> Option<HomeAction> {
        let painter = ui.painter().clone();
        let (top, h, gap) = (606.0, 150.0, 12.0);
        let join_w = 250.0;
        let join_x = 1232.0 - join_w;

        let names: Vec<(String, bool)> = if self.solo {
            vec![("YOU".to_string(), true)]
        } else {
            couch.players.iter().map(|p| (p.name.clone(), p.connected)).collect()
        };
        let n = names.len().max(1) as f32;
        let card_w = ((join_x - gap - 48.0 - gap * (n - 1.0)) / n).min(300.0);
        for (i, (name, online)) in names.iter().enumerate() {
            let rect = c.r(48.0 + i as f32 * (card_w + gap), top, card_w, h);
            let fade = if *online { 1.0 } else { 0.5 };
            kit::poly(&painter, kit::pattern_b(rect, c.v(theme::CHAMFER_LG)), PANEL.gamma_multiply(fade), Stroke::NONE);
            let plate = Rect::from_min_size(rect.min + Vec2::splat(c.v(16.0)), Vec2::splat(c.v(118.0)));
            kit::photo_plate(&painter, plate, name, kit::team_photo(ui.ctx(), name).as_ref(), false);
            let tx = plate.right() + c.v(18.0);
            painter.text(Pos2::new(tx, rect.top() + c.v(18.0)), Align2::LEFT_TOP, name.to_uppercase(), c.cond(800, 30.0), INK.gamma_multiply(fade));
            let status = if self.solo { "SOLO" } else if *online { "ON PHONE" } else { "AWAY" };
            kit::label(&painter, c, Pos2::new(tx, rect.bottom() - c.v(24.0)), Align2::LEFT_CENTER, status, MUTED.gamma_multiply(fade));
        }
        if !self.solo && couch.players.is_empty() {
            let r = c.r(48.0, top, join_x - gap - 48.0, h);
            kit::wrapped(&painter, Pos2::new(r.left(), r.center().y - c.v(12.0)), "Nobody on the couch yet. Scan the code to join from a phone.",
                c.barlow(500, theme::TYPE_BODY), MUTED, r.width(), c.v(26.0));
        }

        // Join card (dashed): QR + URL when the server runs; otherwise a nudge to TEAMS.
        let rect = c.r(join_x, top, join_w, h);
        kit::dashed_rect(&painter, rect, c.v(4.0), c.v(3.0), Stroke::new(1.0, theme::alpha(MUTED, 0.45)));
        let resp = ui.interact(rect, Id::new("home_join"), Sense::click());
        if let Some(url) = couch.url {
            if self.qr.as_ref().map(|(u, _)| u.as_str()) != Some(url) {
                self.qr = kit::qr_modules(url).map(|m| (url.to_string(), m));
            }
            let q = Rect::from_min_size(Pos2::new(rect.left() + c.v(16.0), rect.center().y - c.v(42.0)), Vec2::splat(c.v(84.0)));
            if let Some((_, m)) = &self.qr {
                kit::draw_qr(&painter, q, m);
            }
            let tx = q.right() + c.v(14.0);
            painter.text(Pos2::new(tx, rect.center().y - c.v(22.0)), Align2::LEFT_CENTER, "JOIN ON PHONE", c.cond(800, 20.0), INK);
            painter.text(Pos2::new(tx, rect.center().y + c.v(2.0)), Align2::LEFT_CENTER, url.trim_start_matches("http://"), c.cond(700, 15.0), GOLD);
            painter.text(Pos2::new(tx, rect.center().y + c.v(24.0)), Align2::LEFT_CENTER, "Or add a team here", c.barlow(500, 13.0), MUTED);
        } else {
            let tx = rect.left() + c.v(20.0);
            painter.text(Pos2::new(tx, rect.center().y - c.v(12.0)), Align2::LEFT_CENTER, "PLAY ON PHONES", c.cond(800, 20.0), INK);
            painter.text(Pos2::new(tx, rect.center().y + c.v(14.0)), Align2::LEFT_CENTER, "Pick TEAMS to open the lobby", c.barlow(500, 13.0), MUTED);
        }
        if resp.clicked() && self.solo {
            self.solo = false;
            if couch.url.is_none() {
                return Some(HomeAction::OpenTeams);
            }
        }
        None
    }
}
