//! Shared UI kit — egui ports of the prototype's reusable components
//! (studio.css: .it-ambient, .it-btn, .it-score-chip, .it-quit, .it-loading,
//! .it-summary, .it-confetti). Every in-game screen builds from these so the
//! look stays consistent with the React design and the design tokens.

use egui::{
    Align2, Color32, FontId, Id, Mesh, Pos2, Rect, Response, Rounding, Sense, Shape, Stroke,
    Ui, Vec2,
};

use crate::theme::{
    self, ACCENT_SOFT, APP_BOTTOM, APP_MID, APP_TOP, BLUE_MID, BODY_BG, GOLD, GOLD_HOVER, INK_DIM,
    LINE, ON_ACCENT, RADIUS_PILL, WHITE,
};

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

fn dim(c: Color32, f: f32) -> Color32 {
    Color32::from_rgb(
        (c.r() as f32 * f) as u8,
        (c.g() as f32 * f) as u8,
        (c.b() as f32 * f) as u8,
    )
}

/// cubic ease-out, matches the prototype's cubic-bezier(.2,.8,.2,1) feel.
fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// `.it-enter` mount animation: content slides up 10px and settles over 0.4s.
/// `start` is the time (ctx seconds) the view appeared. Returns the y offset.
pub fn enter_offset(ui: &Ui, start: f64) -> f32 {
    let now = ui.input(|i| i.time);
    let t = ((now - start) / 0.4).clamp(0.0, 1.0) as f32;
    if t < 1.0 {
        ui.ctx().request_repaint();
    }
    (1.0 - ease_out(t)) * 10.0
}

// ---------------------------------------------------------------------------
// Ambient backdrop (.it-ambient + #app radial)
// ---------------------------------------------------------------------------

fn radial(mesh: &mut Mesh, center: Pos2, inner: Color32, rx: f32, ry: f32) {
    let base = mesh.vertices.len() as u32;
    mesh.colored_vertex(center, inner);
    let segs = 72u32;
    for i in 0..=segs {
        let a = i as f32 / segs as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(
            Pos2::new(center.x + a.cos() * rx, center.y + a.sin() * ry),
            Color32::TRANSPARENT,
        );
    }
    for i in 0..segs {
        mesh.add_triangle(base, base + 1 + i, base + 1 + (i + 1) % (segs + 1));
    }
}

/// The breathing navy radial + accent glow + bottom vignette behind every
/// screen. Painted to the background layer so content draws on top.
pub fn ambient(ctx: &egui::Context) {
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());

    painter.rect_filled(screen, Rounding::ZERO, BODY_BG);

    let time = ctx.input(|i| i.time) as f32;
    let breath = 0.5 + 0.5 * (time * 0.55).sin(); // ~11s, .it-breathe
    let r_scale = 0.97 + 0.06 * breath;
    let bright = 0.88 + 0.12 * breath;
    ctx.request_repaint();

    let center = Pos2::new(screen.center().x, screen.top() + screen.height() * 0.24);
    let radius = screen.width().max(screen.height()) * 0.60 * r_scale;

    let mut mesh = Mesh::default();
    // navy halo + brighter core (APP_TOP→APP_MID falloff)
    radial(&mut mesh, center, dim(APP_TOP, bright * 0.92), radius * 1.20, radius * 0.78);
    radial(&mut mesh, center, dim(APP_MID.gamma_multiply(1.6), bright), radius * 0.62, radius * 0.46);
    painter.add(Shape::mesh(mesh));

    // accent breath glow near the top (.it-ambient::before)
    let mut glow = Mesh::default();
    let acc = Color32::from_rgba_premultiplied(
        (GOLD.r() as f32 * (0.10 + 0.06 * breath)) as u8,
        (GOLD.g() as f32 * (0.10 + 0.06 * breath)) as u8,
        (GOLD.b() as f32 * (0.10 + 0.06 * breath)) as u8,
        (40.0 + 24.0 * breath) as u8,
    );
    radial(
        &mut glow,
        Pos2::new(screen.center().x, screen.top() + screen.height() * 0.06),
        acc,
        screen.width() * 0.34,
        screen.height() * 0.24,
    );
    painter.add(Shape::mesh(glow));

    // bottom vignette (.it-ambient::after)
    let mut vig = Mesh::default();
    let h = screen.height() * 0.28;
    let top_y = screen.bottom() - h;
    let clear = Color32::TRANSPARENT;
    let dark = Color32::from_rgba_premultiplied(
        APP_BOTTOM.r() / 3,
        APP_BOTTOM.g() / 3,
        APP_BOTTOM.b() / 3,
        150,
    );
    let i0 = vig.vertices.len() as u32;
    vig.colored_vertex(Pos2::new(screen.left(), top_y), clear);
    vig.colored_vertex(Pos2::new(screen.right(), top_y), clear);
    vig.colored_vertex(Pos2::new(screen.right(), screen.bottom()), dark);
    vig.colored_vertex(Pos2::new(screen.left(), screen.bottom()), dark);
    vig.add_triangle(i0, i0 + 1, i0 + 2);
    vig.add_triangle(i0, i0 + 2, i0 + 3);
    painter.add(Shape::mesh(vig));
}

// ---------------------------------------------------------------------------
// Buttons (.it-btn-primary / .it-btn-ghost)
// ---------------------------------------------------------------------------

fn measure(ui: &Ui, text: &str, font: FontId) -> Vec2 {
    ui.painter()
        .layout_no_wrap(text.to_string(), font, WHITE)
        .size()
}

/// Gold pill button with a hover lift + soft accent glow. Sizes to its label.
pub fn primary_button(ui: &mut Ui, label: &str) -> Response {
    let font = theme::plex_semibold(14.0);
    let pad = Vec2::new(22.0, 12.0);
    let size = measure(ui, label, font.clone()) + pad * 2.0;
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());

    let t = ui.ctx().animate_bool(resp.id, resp.hovered());
    let press = if resp.is_pointer_button_down_on() { 0.97 } else { 1.0 };
    let lift = -2.0 * t;
    let draw = Rect::from_center_size(rect.center() + Vec2::new(0.0, lift), size * press);

    if t > 0.0 {
        ui.painter().rect_stroke(
            draw.expand(4.0 * t),
            Rounding::same(RADIUS_PILL),
            Stroke::new(1.0, ACCENT_SOFT),
        );
    }
    let fill = lerp_color(GOLD, GOLD_HOVER, t);
    ui.painter().rect_filled(draw, Rounding::same(RADIUS_PILL), fill);
    ui.painter().text(
        draw.center(),
        Align2::CENTER_CENTER,
        label,
        font,
        ON_ACCENT,
    );
    resp
}

/// Transparent, hairline-bordered button. Brightens on hover.
pub fn ghost_button(ui: &mut Ui, label: &str) -> Response {
    let font = theme::plex_semibold(14.0);
    let pad = Vec2::new(22.0, 12.0);
    let size = measure(ui, label, font.clone()) + pad * 2.0;
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());

    let t = ui.ctx().animate_bool(resp.id, resp.hovered());
    let border = lerp_color(LINE, INK_DIM, t);
    let text = lerp_color(INK_DIM, WHITE, t);
    ui.painter()
        .rect_stroke(rect, Rounding::same(RADIUS_PILL), Stroke::new(1.0, border));
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, label, font, text);
    resp
}

// ---------------------------------------------------------------------------
// Score chip (.it-score-chip)
// ---------------------------------------------------------------------------

/// Pill chip: tracked-out mono label + bold value. `bump` (0..1) scales it for
/// the score-change pop. Draws within `rect` (right-aligned content).
pub fn score_chip(ui: &Ui, anchor_right: Pos2, label: &str, value: &str, bump: f32) {
    let label_font = theme::mono(theme::TYPE_CHIP_LABEL);
    let value_font = theme::mono_semibold(theme::TYPE_SCORE_VALUE);
    let lab = measure(ui, label, label_font.clone());
    let val = measure(ui, value, value_font.clone());
    let gap = 8.0;
    let pad = Vec2::new(14.0, 7.0);
    let inner_w = lab.x + gap + val.x;
    let size = Vec2::new(inner_w + pad.x * 2.0, lab.y.max(val.y) + pad.y * 2.0);

    let scale = 1.0 + 0.14 * bump;
    let rect = Rect::from_min_size(
        Pos2::new(anchor_right.x - size.x, anchor_right.y - size.y / 2.0),
        size,
    );
    let draw = Rect::from_center_size(rect.center(), size * scale);

    ui.painter().rect_filled(draw, Rounding::same(RADIUS_PILL), BLUE_MID);
    ui.painter()
        .rect_stroke(draw, Rounding::same(RADIUS_PILL), Stroke::new(1.0, LINE));
    let lx = draw.left() + pad.x * scale;
    ui.painter().text(
        Pos2::new(lx, draw.center().y),
        Align2::LEFT_CENTER,
        label,
        label_font,
        INK_DIM,
    );
    ui.painter().text(
        Pos2::new(draw.right() - pad.x * scale, draw.center().y),
        Align2::RIGHT_CENTER,
        value,
        value_font,
        WHITE,
    );
}

// ---------------------------------------------------------------------------
// Quit button (.it-quit) — fixed top-right
// ---------------------------------------------------------------------------

pub fn quit_button(ui: &mut Ui) -> bool {
    let screen = ui.ctx().screen_rect();
    let font = theme::plex_semibold(12.0);
    let label = "Quit";
    let size = measure(ui, label, font.clone()) + Vec2::new(16.0, 8.0) * 2.0;
    let rect = Rect::from_min_size(
        Pos2::new(screen.right() - 18.0 - size.x, screen.top() + 16.0),
        size,
    );
    let resp = ui.interact(rect, Id::new("kit_quit"), Sense::click());
    let t = ui.ctx().animate_bool(resp.id, resp.hovered());
    ui.painter().rect_stroke(
        rect,
        Rounding::same(RADIUS_PILL),
        Stroke::new(1.0, lerp_color(LINE, INK_DIM, t)),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        font,
        lerp_color(INK_DIM, WHITE, t),
    );
    resp.clicked()
}

// ---------------------------------------------------------------------------
// Loader (.it-loading) — three bouncing dots + italic serif label
// ---------------------------------------------------------------------------

pub fn loader(ui: &mut Ui, label: &str) {
    let ctx = ui.ctx().clone();
    ctx.request_repaint();
    let time = ui.input(|i| i.time) as f32;

    let avail = ui.available_size();
    ui.add_space((avail.y - 80.0) / 2.0);
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(70.0, 24.0), Sense::hover());
        for k in 0..3 {
            let phase = time * (1.0 / 1.2) * std::f32::consts::TAU - k as f32 * 0.95;
            let s = (phase.sin() * 0.5 + 0.5).max(0.0);
            let cx = rect.left() + 14.0 + k as f32 * 21.0;
            let cy = rect.center().y - s * 10.0;
            let alpha = (0.25 + 0.75 * s) as f32;
            ui.painter().circle_filled(
                Pos2::new(cx, cy),
                7.0,
                GOLD.gamma_multiply(alpha),
            );
        }
        ui.add_space(14.0);
        ui.label(
            egui::RichText::new(label)
                .font(theme::spectral_italic(22.0))
                .color(INK_DIM),
        );
    });
}

// ---------------------------------------------------------------------------
// Summary (.it-summary) — kicker, big score, sub, stats, actions + confetti
// ---------------------------------------------------------------------------

pub enum SummaryAction {
    Restart,
    Quit,
    None,
}

pub struct Stat<'a> {
    pub value: String,
    pub label: &'a str,
}

#[allow(clippy::too_many_arguments)]
pub fn summary(
    ui: &mut Ui,
    kicker: &str,
    score_text: &str,
    sub: &str,
    stats: &[Stat],
    enter_start: f64,
) -> SummaryAction {
    confetti(ui);

    let mut action = SummaryAction::None;
    let avail = ui.available_size();
    let now = ui.input(|i| i.time);
    let pop = ease_out((((now - enter_start) / 0.6).clamp(0.0, 1.0)) as f32);

    ui.add_space((avail.y - 360.0).max(16.0) / 2.0);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(kicker)
                .font(theme::mono(12.0))
                .color(GOLD),
        );
        ui.add_space(6.0);
        // big score with verdict-pop scale
        let score_font = theme::mono_semibold(72.0 * (0.5 + 0.5 * pop));
        ui.label(
            egui::RichText::new(score_text)
                .font(score_font)
                .color(GOLD),
        );
        ui.label(
            egui::RichText::new(sub)
                .font(theme::mono(11.0))
                .color(INK_DIM),
        );
        ui.add_space(26.0);

        ui.horizontal(|ui| {
            let total_w: f32 = stats.len() as f32 * 120.0;
            ui.add_space((avail.x - total_w) / 2.0);
            for s in stats {
                ui.allocate_ui(Vec2::new(120.0, 60.0), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(&s.value)
                                .font(theme::spectral(32.0))
                                .color(WHITE),
                        );
                        ui.label(
                            egui::RichText::new(s.label)
                                .font(theme::mono(10.5))
                                .color(INK_DIM),
                        );
                    });
                });
            }
        });

        ui.add_space(30.0);
        ui.horizontal(|ui| {
            let (pw, gw) = (
                measure(ui, "Play again", theme::plex_semibold(14.0)).x + 44.0,
                measure(ui, "Back to menu", theme::plex_semibold(14.0)).x + 44.0,
            );
            ui.add_space((avail.x - pw - gw - 14.0) / 2.0);
            if primary_button(ui, "Play again").clicked() {
                action = SummaryAction::Restart;
            }
            ui.add_space(14.0);
            if ghost_button(ui, "Back to menu").clicked() {
                action = SummaryAction::Quit;
            }
        });
    });

    action
}

// ---------------------------------------------------------------------------
// Overlay pieces (.it-verdict / .it-pop / .it-answer-reveal) — centered column
// ---------------------------------------------------------------------------

fn ease_out_pub(t: f32) -> f32 {
    ease_out(t)
}

/// Centered verdict label (with pop scale) and, below it, an optional points
/// pop (slide-up + fade). `start` is the time the result appeared.
pub fn overlay_verdict(
    ui: &Ui,
    cx: f32,
    label_cy: f32,
    start: f64,
    label: &str,
    color: Color32,
    delta_text: Option<String>,
) {
    let now = ui.input(|i| i.time);
    let t = (((now - start) / 0.5).clamp(0.0, 1.0)) as f32;
    let pop = ease_out_pub(t);
    if t < 1.0 {
        ui.ctx().request_repaint();
    }
    let painter = ui.painter();
    painter.text(
        Pos2::new(cx, label_cy),
        Align2::CENTER_CENTER,
        label,
        theme::spectral(44.0 * (0.5 + 0.5 * pop)),
        color,
    );
    if let Some(d) = delta_text {
        let dy = (1.0 - pop) * 18.0;
        painter.text(
            Pos2::new(cx, label_cy + 36.0 + dy),
            Align2::CENTER_CENTER,
            d,
            theme::mono_semibold(22.0),
            color.gamma_multiply(pop),
        );
    }
}

/// Centered `.it-answer-reveal`: a tracked-out kicker over the answer text.
pub fn answer_reveal(ui: &Ui, cx: f32, top_y: f32, kicker: &str, answer: &str) {
    let painter = ui.painter();
    painter.text(
        Pos2::new(cx, top_y),
        Align2::CENTER_CENTER,
        kicker,
        theme::mono(11.0),
        INK_DIM,
    );
    painter.text(
        Pos2::new(cx, top_y + 24.0),
        Align2::CENTER_CENTER,
        answer,
        theme::spectral(22.0),
        WHITE,
    );
}

/// Centered serif clue/question text wrapped to `max_w`. Returns its height.
pub fn centered_question(ui: &Ui, cx: f32, top_y: f32, text: &str, size: f32, max_w: f32) -> f32 {
    let mut job = egui::text::LayoutJob::default();
    job.halign = egui::Align::Center;
    job.wrap.max_width = max_w;
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: theme::spectral_medium(size),
            color: WHITE,
            line_height: Some(size * 1.34),
            ..Default::default()
        },
    );
    let galley = ui.fonts(|f| f.layout_job(job));
    let h = galley.size().y;
    // With halign: Center the galley's rows are centered about its origin x,
    // so draw at cx directly (not cx - w/2).
    ui.painter().galley(Pos2::new(cx, top_y), galley, WHITE);
    h
}

/// Centered `prompt + single-line input` row (.it-prompt + .it-input).
/// Returns true if Enter was pressed to submit.
pub fn input_row(
    ui: &mut Ui,
    cx: f32,
    cy: f32,
    prompt: &str,
    buf: &mut String,
    field_w: f32,
    focus: bool,
) -> bool {
    let pfont = theme::mono(12.0);
    let pw = measure(ui, prompt, pfont.clone()).x;
    let gap = 12.0;
    let row_w = pw + gap + field_w;
    let left = cx - row_w / 2.0;
    ui.painter().text(
        Pos2::new(left, cy),
        Align2::LEFT_CENTER,
        prompt,
        pfont,
        GOLD,
    );
    let field = Rect::from_min_size(Pos2::new(left + pw + gap, cy - 21.0), Vec2::new(field_w, 42.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(field)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let resp = child.add(
        egui::TextEdit::singleline(buf)
            .desired_width(field_w)
            .font(theme::mono(17.0))
            .margin(egui::Margin::symmetric(14.0, 11.0))
            .text_color(WHITE),
    );
    if focus {
        resp.request_focus();
    }
    resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}

/// Centered wrapped paragraph in an arbitrary font/color. Returns its height.
pub fn centered_para(
    ui: &Ui,
    cx: f32,
    top_y: f32,
    text: &str,
    font: FontId,
    color: Color32,
    max_w: f32,
) -> f32 {
    let mut job = egui::text::LayoutJob::default();
    job.halign = egui::Align::Center;
    job.wrap.max_width = max_w;
    let line_height = font.size * 1.34;
    job.append(
        text,
        0.0,
        egui::TextFormat { font_id: font, color, line_height: Some(line_height), ..Default::default() },
    );
    let galley = ui.fonts(|f| f.layout_job(job));
    let h = galley.size().y;
    ui.painter().galley(Pos2::new(cx, top_y), galley, color);
    h
}

/// A single primary button placed centered at `cy`, sized to its label.
pub fn centered_primary(ui: &mut Ui, cx: f32, cy: f32, label: &str) -> bool {
    let sz = measure(ui, label, theme::plex_semibold(14.0)) + Vec2::new(44.0, 24.0);
    let rect = Rect::from_center_size(Pos2::new(cx, cy + sz.y / 2.0), sz);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    primary_button(&mut child, label).clicked()
}

// ---------------------------------------------------------------------------
// Confetti (.it-confetti) — 32 deterministic falling pieces
// ---------------------------------------------------------------------------

pub fn confetti(ui: &Ui) {
    let ctx = ui.ctx();
    ctx.request_repaint();
    let screen = ctx.screen_rect();
    let painter = ui.painter();
    let time = ui.input(|i| i.time) as f32;

    let colors = [GOLD, GOLD_HOVER, theme::GREEN_CORRECT, WHITE];
    for i in 0..32u32 {
        // deterministic pseudo-randoms from index
        let r1 = frac((i as f32) * 12.9898_f32.sin() * 43758.5453);
        let r2 = frac((i as f32 + 1.0) * 78.233_f32.sin() * 12543.123);
        let r3 = frac((i as f32 + 2.0) * 39.425_f32.sin() * 9281.77);
        let dur = 2.4 + r2 * 1.8;
        let delay = r1 * 2.6;
        let p = frac((time + delay) / dur); // 0..1 progress
        let x = screen.left() + r1 * screen.width() + (r3 * 2.0 - 1.0) * 70.0 * p;
        let y = screen.top() - 20.0 + p * (screen.height() + 40.0);
        let w = 5.0 + r2 * 5.0;
        let h = 8.0 + r3 * 8.0;
        let alpha = if p < 0.09 {
            p / 0.09
        } else {
            (1.0 - p).clamp(0.0, 1.0)
        };
        let col = colors[(i % 4) as usize].gamma_multiply(alpha);
        let rot = r3 * std::f32::consts::TAU + p * 6.0;
        let rect = Rect::from_center_size(Pos2::new(x, y), Vec2::new(w, h));
        // cheap rotation: draw a filled quad
        let c = rect.center();
        let (cos, sin) = (rot.cos(), rot.sin());
        let corner = |dx: f32, dy: f32| {
            Pos2::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
        };
        let pts = vec![
            corner(-w / 2.0, -h / 2.0),
            corner(w / 2.0, -h / 2.0),
            corner(w / 2.0, h / 2.0),
            corner(-w / 2.0, h / 2.0),
        ];
        painter.add(Shape::convex_polygon(pts, col, Stroke::NONE));
    }
}

fn frac(x: f32) -> f32 {
    let f = x.fract().abs();
    if f.is_nan() {
        0.0
    } else {
        f
    }
}
