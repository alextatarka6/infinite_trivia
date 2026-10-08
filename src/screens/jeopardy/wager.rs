//! Shared layout for the wagering screens — "04 Daily Double" in the Roster
//! hi-fi, reused for Final Jeopardy: a stinger band, the wager panel (gold
//! inset field, LOCK IT, quick chips), the clue + response, and the verdict.

use egui::{Align2, Key, Pos2, Rect, Stroke, Vec2};

use crate::screens::kit::{self, Btn, Canvas, RosterEntry};
use crate::theme::{self, BAD, BG, GOLD, GOOD, INK, MUTED, ON_BAD, ON_GOOD, PANEL_HI};

/// The big band across the top of the screen. `full` (0..1) blends from the
/// tall wager-phase band to the slim header band used once the clue shows.
/// `enter` is seconds since the screen appeared (the band sweeps in).
pub fn band(ui: &egui::Ui, c: &Canvas, title: &str, full: f32, enter: f32) {
    let painter = ui.painter();
    let slide = (1.0 - kit::ease_out(enter / 0.35)) * c.screen.width();
    let lerp = |a: f32, b: f32| b + (a - b) * full;
    let center = c.p(640.0, lerp(255.0, 58.0)) + Vec2::new(-slide, 0.0);
    let thick = c.v(lerp(190.0, 64.0));
    let deg = lerp(-8.5, -2.0);
    let a = deg.to_radians();
    let normal = Vec2::new(-a.sin(), a.cos());
    kit::stinger(painter, c.screen, center + normal * (thick / 2.0 + c.v(lerp(16.0, 6.0))), deg, c.v(lerp(18.0, 6.0)), PANEL_HI, None);
    kit::stinger(painter, c.screen, center, deg, thick, GOLD, Some((title, c.cond(800, lerp(150.0, 44.0)), BG)));
}

/// The wager panel. Returns the locked wager (clamped to `min..=max`).
#[allow(clippy::too_many_arguments)]
pub fn wager_panel(
    ui: &mut egui::Ui,
    c: &Canvas,
    heading: &str,
    input: &mut String,
    min: i32,
    max: i32,
    focus: bool,
    id: &str,
) -> Option<i32> {
    let painter = ui.painter().clone();
    kit::label(&painter, c, c.p(120.0, 486.0), Align2::LEFT_CENTER, heading, MUTED);

    // Gold inset field with a muted "$" prefix.
    let field = c.r(120.0, 505.0, 360.0, 84.0);
    painter.text(Pos2::new(field.left() + c.v(22.0), field.center().y), Align2::LEFT_CENTER, "$", c.cond(800, 56.0), theme::alpha(MUTED, 0.9));
    let digits_rect = Rect::from_min_max(Pos2::new(field.left() + c.v(46.0), field.top()), field.max);
    input.retain(|ch| ch.is_ascii_digit());
    let entered = kit::answer_field_bare(ui, digits_rect, input, focus, c.v(56.0), id);
    painter.rect_stroke(field, 0.0, Stroke::new(c.v(2.0), GOLD));

    let lock = c.r(488.0, 505.0, 129.0, 84.0);
    let clicked = kit::button(ui, c, lock, "LOCK IT", Btn::Primary, (id, "lock")).clicked();
    kit::wrapped(&painter, c.p(120.0, 600.0), &format!("You can bet {} up to {}", kit::money(min), kit::money(max)),
        c.barlow(500, 16.0), MUTED, c.v(480.0), c.v(20.0));

    // Quick chips, right side.
    let chips: [(&str, i32, Btn); 4] = [
        ("$500", 500, Btn::Secondary),
        ("$1,000", 1000, Btn::Secondary),
        ("HALF", max / 2, Btn::Secondary),
        ("ALL IN", max, Btn::Bad),
    ];
    let mut x = 1160.0;
    for (label, amount, kind) in chips.iter().rev() {
        let w = (kit::text_width(&painter, label, c.cond(800, 17.0), 0.08) / c.s + 32.0).max(70.0);
        x -= w;
        if kit::button(ui, c, c.r(x, 576.0, w, 42.0), label, *kind, (id, *label)).clicked() {
            *input = (*amount).clamp(min, max).to_string();
        }
        x -= 6.0;
    }

    if entered || clicked {
        let v = input.trim().parse::<i32>().unwrap_or(max);
        return Some(v.clamp(min, max));
    }
    None
}

/// The wagering team: plate + name + gold score line.
pub fn team_line(ui: &egui::Ui, c: &Canvas, name: &str, score: i32, note: &str) {
    let painter = ui.painter();
    let plate = c.r(120.0, 680.0, 72.0, 72.0);
    kit::photo_plate(painter, plate, name, kit::team_photo(ui.ctx(), name).as_ref(), false);
    painter.text(c.p(208.0, 706.0), Align2::LEFT_CENTER, name.to_uppercase(), c.cond(800, 28.0), INK);
    kit::text_tracked(painter, c.p(208.0, 734.0), Align2::LEFT_CENTER, &format!("{} · {}", kit::money(score), note),
        c.cond(700, 17.0), GOLD, 0.06);
}

/// Clue text + response field + SUBMIT, below the slim band. Returns true on submit.
pub fn question(ui: &mut egui::Ui, c: &Canvas, category: &str, text: &str, input: &mut String, focus: bool, id: &str) -> bool {
    let painter = ui.painter().clone();
    kit::label(&painter, c, c.p(80.0, 140.0), Align2::LEFT_CENTER, &category.to_uppercase(), GOLD);
    let q = text.to_uppercase();
    let max_w = c.v(1000.0);
    let size = kit::fit_size(&painter, &q, 600, c.v(theme::TYPE_CLUE), c.v(30.0), max_w, c.v(240.0), 1.15);
    kit::wrapped(&painter, c.p(80.0, 162.0), &q, theme::cond(600, size), INK, max_w, size * 1.15);
    kit::label(&painter, c, c.p(80.0, 452.0), Align2::LEFT_CENTER, "YOUR RESPONSE", MUTED);
    let submitted = kit::answer_field(ui, c.r(80.0, 470.0, 480.0, 58.0), input, focus, c.v(28.0));
    let sb = c.r(568.0, 470.0, 150.0, 58.0);
    let clicked = kit::button(ui, c, sb, "SUBMIT", Btn::Primary, (id, "submit")).clicked();
    kit::key_hint(&painter, c, Pos2::new(sb.right() + c.v(12.0), sb.center().y), Align2::LEFT_CENTER, "ENTER ↵");
    submitted || clicked
}

/// Verdict band + correct response + continue button. Returns true to continue.
#[allow(clippy::too_many_arguments)]
pub fn result(ui: &mut egui::Ui, c: &Canvas, since_result: f32, correct: bool, delta: i32, answer: &str, extra: Option<&str>, next: &str, id: &str) -> bool {
    let painter = ui.painter().clone();
    let t = kit::ease_out(since_result / 0.3);
    let band = c.r(80.0, 170.0, 560.0, 92.0);
    let band = Rect::from_center_size(band.center(), Vec2::new(band.width(), band.height() * t.max(0.02)));
    let (fill, ink, word) = if correct { (GOOD, ON_GOOD, "CORRECT") } else { (BAD, ON_BAD, "WRONG") };
    kit::poly(&painter, kit::pattern_a(band, c.v(theme::CHAMFER_MD)), fill, Stroke::NONE);
    if t > 0.6 {
        painter.text(Pos2::new(band.left() + c.v(26.0), band.center().y), Align2::LEFT_CENTER, word, c.cond(800, 60.0), ink);
        let d = if delta >= 0 { format!("+{}", kit::money(delta)) } else { kit::money(delta) };
        painter.text(Pos2::new(band.right() - c.v(26.0), band.center().y), Align2::RIGHT_CENTER, d, c.cond(800, 40.0), ink);
    }
    kit::label(&painter, c, c.p(80.0, 300.0), Align2::LEFT_CENTER, "CORRECT RESPONSE", MUTED);
    let r = kit::wrapped(&painter, c.p(80.0, 314.0), &answer.to_uppercase(), c.cond(800, 48.0), INK, c.v(900.0), c.v(50.0));
    let mut y = (r.bottom() - c.origin.y) / c.s + 20.0;
    if let Some(extra) = extra {
        kit::text_tracked(&painter, c.p(80.0, y + 10.0), Align2::LEFT_CENTER, extra, c.cond(800, 26.0), GOLD, 0.04);
        y += 44.0;
    }
    let label = next.to_uppercase();
    let size = kit::button_size(&painter, c, &label, theme::TYPE_BUTTON, Vec2::new(24.0, 14.0));
    let br = Rect::from_min_size(c.p(80.0, y + 10.0), size);
    let go = kit::button(ui, c, br, &label, Btn::Primary, (id, "next")).clicked() || ui.input(|i| i.key_pressed(Key::Enter));
    kit::key_hint(&painter, c, Pos2::new(br.right() + c.v(12.0), br.center().y), Align2::LEFT_CENTER, "ENTER ↵");
    go
}

/// Roster row for the wagering screens (only the wagering team is active).
pub fn roster(ui: &egui::Ui, c: &Canvas, teams: &[(String, i32)], active: usize) {
    if teams.len() < 2 {
        return;
    }
    let entries: Vec<RosterEntry> = teams
        .iter()
        .map(|(n, s)| {
            let mut e = RosterEntry::new(n, *s);
            e.money = true;
            e
        })
        .collect();
    kit::roster(ui, c, 40.0, 1200.0, 772.0, &entries, Some(active), true);
}
