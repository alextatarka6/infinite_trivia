//! Shared UI kit — the "Roster" design system (design/roster/Roster Hi-fi.dc.html).
//!
//! Everything is a flat fill, a convex polygon or a line segment: no gradients,
//! glows or rounded corners. Screens are laid out on a 1280×800 design canvas
//! ([`Canvas`]) so sizes can be copied straight from the HTML's inline styles.
//!
//! Shapes: chamfered rects (pattern A = top-left + bottom-right, B = bottom-right
//! only), slants (parallelograms) and tags (right edge slanted). Textures are
//! line patterns clipped to those polygons, one per mode.

use egui::{
    epaint::TextShape, text::LayoutJob, Align2, Color32, FontId, Id, Pos2, Rect, Response, Sense,
    Shape, Stroke, TextFormat, TextureHandle, Ui, Vec2,
};

use crate::game::session::{monogram, Crest};
use crate::theme::{
    self, BAD, BG, CANVAS_H, CANVAS_W, CHAMFER_LG, CHAMFER_MD, CHAMFER_SM, GOLD, GOLD_HI, GOOD,
    INK, LINE, MUTED, ON_BAD, ON_GOLD, ON_GOOD, PANEL, PANEL_HI, SLANT_TAG,
};

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

/// Cubic ease-out.
pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// `$1,800` / `−$400`.
pub fn money(n: i32) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    format!("{}${}", if n < 0 { "−" } else { "" }, out)
}

/// Seconds since `start`, keeping the frame clock running while `< until`.
pub fn since(ui: &Ui, start: f64, until: f32) -> f32 {
    let t = (ui.input(|i| i.time) - start) as f32;
    if t < until {
        ui.ctx().request_repaint();
    }
    t
}

// ---------------------------------------------------------------------------
// Canvas — the 1280×800 design space mapped onto the window
// ---------------------------------------------------------------------------

/// Uniformly scales the design canvas to fit the window and centers it.
/// `c.p(x, y)` / `c.r(x, y, w, h)` take design pixels; `c.v(n)` scales a length.
#[derive(Clone, Copy)]
pub struct Canvas {
    pub s: f32,
    pub origin: Pos2,
    /// The full window (for full-bleed elements like the stinger band).
    pub screen: Rect,
}

impl Canvas {
    pub fn new(ctx: &egui::Context) -> Self {
        let screen = ctx.screen_rect();
        let s = (screen.width() / CANVAS_W).min(screen.height() / CANVAS_H);
        let origin = screen.center() - Vec2::new(CANVAS_W, CANVAS_H) * s / 2.0;
        Self { s, origin, screen }
    }
    pub fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + Vec2::new(x, y) * self.s
    }
    pub fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.p(x, y), Vec2::new(w, h) * self.s)
    }
    pub fn v(&self, n: f32) -> f32 {
        n * self.s
    }
    /// Barlow Condensed, scaled.
    pub fn cond(&self, weight: u16, size: f32) -> FontId {
        theme::cond(weight, size * self.s)
    }
    /// Barlow, scaled.
    pub fn barlow(&self, weight: u16, size: f32) -> FontId {
        theme::barlow(weight, size * self.s)
    }
}

// ---------------------------------------------------------------------------
// Polygons: chamfers, slants, tags
// ---------------------------------------------------------------------------

/// Rect with 45° clipped corners; each value is the cut size (0 = square).
pub fn chamfer_pts(r: Rect, tl: f32, tr: f32, br: f32, bl: f32) -> Vec<Pos2> {
    let mut pts = Vec::with_capacity(8);
    let mut push = |p: Pos2| {
        if pts.last() != Some(&p) {
            pts.push(p);
        }
    };
    push(Pos2::new(r.left() + tl, r.top()));
    push(Pos2::new(r.right() - tr, r.top()));
    push(Pos2::new(r.right(), r.top() + tr));
    push(Pos2::new(r.right(), r.bottom() - br));
    push(Pos2::new(r.right() - br, r.bottom()));
    push(Pos2::new(r.left() + bl, r.bottom()));
    push(Pos2::new(r.left(), r.bottom() - bl));
    push(Pos2::new(r.left(), r.top() + tl));
    if pts.first() == pts.last() {
        pts.pop();
    }
    pts
}

/// Chamfer pattern A: top-left + bottom-right (buttons, tiles, photo plates).
pub fn pattern_a(r: Rect, c: f32) -> Vec<Pos2> {
    chamfer_pts(r, c, 0.0, c, 0.0)
}

/// Chamfer pattern B: bottom-right only (roster / team cards).
pub fn pattern_b(r: Rect, c: f32) -> Vec<Pos2> {
    chamfer_pts(r, 0.0, 0.0, c, 0.0)
}

/// Parallelogram leaning right: top edge shifted `off` to the right.
pub fn slant_pts(r: Rect, off: f32) -> Vec<Pos2> {
    vec![
        Pos2::new(r.left() + off, r.top()),
        r.right_top(),
        Pos2::new(r.right() - off, r.bottom()),
        r.left_bottom(),
    ]
}

/// Tag shape: square left edge, slanted right edge.
pub fn tag_pts(r: Rect, off: f32) -> Vec<Pos2> {
    vec![r.left_top(), r.right_top(), Pos2::new(r.right() - off, r.bottom()), r.left_bottom()]
}

pub fn poly(painter: &egui::Painter, pts: Vec<Pos2>, fill: Color32, stroke: Stroke) {
    painter.add(Shape::convex_polygon(pts, fill, stroke));
}

/// `chamfer_rect(rect, tl, tr, br, bl, fill, stroke)` from the handoff.
#[allow(clippy::too_many_arguments, dead_code)]
pub fn chamfer_rect(
    painter: &egui::Painter,
    rect: Rect,
    tl: f32,
    tr: f32,
    br: f32,
    bl: f32,
    fill: Color32,
    stroke: Stroke,
) {
    poly(painter, chamfer_pts(rect, tl, tr, br, bl), fill, stroke);
}

#[allow(dead_code)]
pub fn slant_rect(painter: &egui::Painter, rect: Rect, offset: f32, fill: Color32, stroke: Stroke) {
    poly(painter, slant_pts(rect, offset), fill, stroke);
}

/// Shrink a convex polygon toward its centroid-free inset by moving each edge
/// inward by `d` (used for inset strokes so borders stay inside the shape).
pub fn inset(pts: &[Pos2], d: f32) -> Vec<Pos2> {
    let n = pts.len();
    // Lines offset inward (polygon is clockwise in screen space -> inward normal is right-hand).
    let lines: Vec<(Pos2, Vec2)> = (0..n)
        .map(|i| {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            let dir = (b - a).normalized();
            let normal = Vec2::new(-dir.y, dir.x);
            (a + normal * d, dir)
        })
        .collect();
    (0..n)
        .map(|i| {
            let (p1, d1) = lines[(i + n - 1) % n];
            let (p2, d2) = lines[i];
            let cross = d1.x * d2.y - d1.y * d2.x;
            if cross.abs() < 1e-5 {
                return p2;
            }
            let t = ((p2.x - p1.x) * d2.y - (p2.y - p1.y) * d2.x) / cross;
            p1 + d1 * t
        })
        .collect()
}

/// A stroke drawn inside the shape's edge (CSS `box-shadow: inset 0 0 0 Npx`).
pub fn inner_stroke(painter: &egui::Painter, pts: &[Pos2], width: f32, color: Color32) {
    let p = inset(pts, width / 2.0);
    painter.add(Shape::closed_line(p, Stroke::new(width, color)));
}

// ---------------------------------------------------------------------------
// Convex clipping (for textures inside chamfered / slanted shapes)
// ---------------------------------------------------------------------------

fn inside(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    // > 0 when p is on the interior side of edge a->b (clockwise polygon, y down).
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

/// Clip segment a-b to a convex polygon (Cyrus–Beck).
fn clip_segment(pts: &[Pos2], a: Pos2, b: Pos2) -> Option<(Pos2, Pos2)> {
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    let d = b - a;
    for i in 0..pts.len() {
        let (e0, e1) = (pts[i], pts[(i + 1) % pts.len()]);
        let fa = inside(a, e0, e1);
        let fb = inside(b, e0, e1);
        if fa < 0.0 && fb < 0.0 {
            return None;
        }
        if (fa < 0.0) != (fb < 0.0) {
            let t = fa / (fa - fb);
            if fa < 0.0 { t0 = t0.max(t) } else { t1 = t1.min(t) }
        }
    }
    (t0 < t1).then(|| (a + d * t0, a + d * t1))
}

/// Clip a convex polygon to another convex polygon (Sutherland–Hodgman).
fn clip_poly(subject: &[Pos2], clip: &[Pos2]) -> Vec<Pos2> {
    let mut out = subject.to_vec();
    for i in 0..clip.len() {
        let (e0, e1) = (clip[i], clip[(i + 1) % clip.len()]);
        let input = std::mem::take(&mut out);
        for j in 0..input.len() {
            let cur = input[j];
            let prev = input[(j + input.len() - 1) % input.len()];
            let (fc, fp) = (inside(cur, e0, e1), inside(prev, e0, e1));
            if fc >= 0.0 {
                if fp < 0.0 {
                    out.push(prev + (cur - prev) * (fp / (fp - fc)));
                }
                out.push(cur);
            } else if fp >= 0.0 {
                out.push(prev + (cur - prev) * (fp / (fp - fc)));
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
}

fn bounds(pts: &[Pos2]) -> Rect {
    Rect::from_points(pts)
}

// ---------------------------------------------------------------------------
// Textures — one per mode, thin line patterns clipped to a shape
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Texture {
    /// Jeopardy: square grid.
    Grid,
    /// General Trivia: 45° hatch.
    Hatch,
    /// Quizbowl: horizontal scanlines.
    Scan,
    /// Mix: checker.
    Checker,
}

impl Texture {
    pub fn pitch(self) -> f32 {
        match self {
            Texture::Grid => theme::TEXTURE_GRID_PITCH,
            Texture::Hatch => theme::TEXTURE_HATCH_PITCH,
            Texture::Scan => theme::TEXTURE_SCAN_PITCH,
            Texture::Checker => theme::TEXTURE_CHECKER_PITCH,
        }
    }
    /// The token alpha of MUTED used on cards.
    pub fn alpha(self) -> f32 {
        match self {
            Texture::Grid => theme::TEXTURE_GRID_ALPHA,
            Texture::Hatch => theme::TEXTURE_HATCH_ALPHA,
            Texture::Scan => theme::TEXTURE_SCAN_ALPHA,
            Texture::Checker => theme::TEXTURE_CHECKER_ALPHA,
        }
    }
}

impl From<Crest> for Texture {
    fn from(c: Crest) -> Self {
        match c {
            Crest::Hatch => Texture::Hatch,
            Crest::Grid => Texture::Grid,
            Crest::Scan => Texture::Scan,
            Crest::Checker => Texture::Checker,
        }
    }
}

/// Draw `kind` inside the convex polygon `clip`. `pitch` is in screen pixels.
pub fn texture(painter: &egui::Painter, clip: &[Pos2], kind: Texture, pitch: f32, color: Color32) {
    let b = bounds(clip);
    let stroke = Stroke::new(1.0, color);
    let seg = |a: Pos2, z: Pos2| {
        if let Some((p, q)) = clip_segment(clip, a, z) {
            painter.line_segment([p, q], stroke);
        }
    };
    match kind {
        Texture::Grid => {
            let mut x = b.left() + pitch;
            while x < b.right() {
                seg(Pos2::new(x, b.top()), Pos2::new(x, b.bottom()));
                x += pitch;
            }
            let mut y = b.top() + pitch;
            while y < b.bottom() {
                seg(Pos2::new(b.left(), y), Pos2::new(b.right(), y));
                y += pitch;
            }
        }
        Texture::Scan => {
            let mut y = b.top() + pitch;
            while y < b.bottom() {
                seg(Pos2::new(b.left(), y), Pos2::new(b.right(), y));
                y += pitch;
            }
        }
        Texture::Hatch => {
            // Lines running top-left -> bottom-right, spaced `pitch` apart horizontally.
            let h = b.height();
            let mut x = b.left() - h;
            while x < b.right() {
                seg(Pos2::new(x, b.top()), Pos2::new(x + h, b.bottom()));
                x += pitch;
            }
        }
        Texture::Checker => {
            let fill = color;
            let (mut row, mut y) = (0, b.top());
            while y < b.bottom() {
                let (mut col, mut x) = (0, b.left());
                while x < b.right() {
                    if (row + col) % 2 == 0 {
                        let cell = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(pitch));
                        let pts = clip_poly(&[cell.left_top(), cell.right_top(), cell.right_bottom(), cell.left_bottom()], clip);
                        if pts.len() >= 3 {
                            painter.add(Shape::convex_polygon(pts, fill, Stroke::NONE));
                        }
                    }
                    x += pitch;
                    col += 1;
                }
                y += pitch;
                row += 1;
            }
        }
    }
}

/// Flat screen background, optionally with the mode's texture at 5% (question screens).
/// The CentralPanel draws into the same background layer, so call this first:
/// anything painted before it is covered.
pub fn backdrop(ctx: &egui::Context, tex: Option<Texture>) {
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(screen, 0.0, BG);
    if let Some(kind) = tex {
        let c = Canvas::new(ctx);
        let pts = vec![screen.left_top(), screen.right_top(), screen.right_bottom(), screen.left_bottom()];
        texture(&painter, &pts, kind, c.v(kind.pitch()), theme::alpha(MUTED, theme::TEXTURE_SCREEN_ALPHA));
    }
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

/// Text with letter-spacing (`tracking` as a fraction of the font size, e.g.
/// 0.2 for CSS `letter-spacing: .2em`). Returns the drawn rect.
pub fn text_tracked(
    painter: &egui::Painter,
    pos: Pos2,
    align: Align2,
    text: &str,
    font: FontId,
    color: Color32,
    tracking: f32,
) -> Rect {
    let galley = tracked_galley(painter, text, font, color, tracking);
    let rect = align.anchor_size(pos, galley.size());
    painter.galley(rect.min, galley, color);
    rect
}

pub fn tracked_galley(
    painter: &egui::Painter,
    text: &str,
    font: FontId,
    color: Color32,
    tracking: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::default();
    let spacing = font.size * tracking;
    job.append(text, 0.0, TextFormat { font_id: font, color, extra_letter_spacing: spacing, ..Default::default() });
    painter.layout_job(job)
}

pub fn text_width(painter: &egui::Painter, text: &str, font: FontId, tracking: f32) -> f32 {
    tracked_galley(painter, text, font, INK, tracking).size().x
}

/// Left-aligned wrapped paragraph. Returns its rect.
pub fn wrapped(
    painter: &egui::Painter,
    top_left: Pos2,
    text: &str,
    font: FontId,
    color: Color32,
    max_w: f32,
    line_height: f32,
) -> Rect {
    let mut job = LayoutJob::default();
    job.wrap.max_width = max_w;
    job.append(text, 0.0, TextFormat { line_height: Some(line_height), font_id: font, color, ..Default::default() });
    let galley = painter.layout_job(job);
    let rect = Rect::from_min_size(top_left, galley.size());
    painter.galley(top_left, galley, color);
    rect
}

/// Height `text` would take if wrapped at `max_w`.
pub fn wrapped_height(painter: &egui::Painter, text: &str, font: FontId, max_w: f32, line_height: f32) -> f32 {
    let mut job = LayoutJob::default();
    job.wrap.max_width = max_w;
    job.append(text, 0.0, TextFormat { line_height: Some(line_height), font_id: font, ..Default::default() });
    painter.layout_job(job).size().y
}

/// Largest font size (stepping down from `size`) at which `text` fits in `max_h`.
pub fn fit_size(painter: &egui::Painter, text: &str, weight: u16, size: f32, min: f32, max_w: f32, max_h: f32, lh: f32) -> f32 {
    let mut s = size;
    while s > min && wrapped_height(painter, text, theme::cond(weight, s), max_w, s * lh) > max_h {
        s -= 2.0;
    }
    s
}

/// Text rotated by `angle` radians about its own center.
pub fn text_rotated(painter: &egui::Painter, center: Pos2, text: &str, font: FontId, color: Color32, tracking: f32, angle: f32) {
    let galley = tracked_galley(painter, text, font, color, tracking);
    let half = galley.size() / 2.0;
    let rot = egui::emath::Rot2::from_angle(angle);
    let pos = center - rot * half;
    painter.add(TextShape::new(pos, galley, color).with_angle(angle));
}

/// Small tracked caps label (DIFFICULTY, ON THE COUCH, CORRECT).
pub fn label(painter: &egui::Painter, c: &Canvas, pos: Pos2, align: Align2, text: &str, color: Color32) -> Rect {
    text_tracked(painter, pos, align, text, c.cond(700, theme::TYPE_LABEL), color, 0.2)
}

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    /// Gold fill.
    Primary,
    /// Panel fill.
    Secondary,
    /// No fill, muted text (HOME, REVEAL R).
    Ghost,
    /// Good fill (CORRECT).
    Good,
    /// Bad fill (WRONG, ALL IN).
    Bad,
}

impl Btn {
    fn colors(self, hover: f32) -> (Color32, Color32) {
        match self {
            Btn::Primary => (lerp_color(GOLD, GOLD_HI, hover), ON_GOLD),
            Btn::Secondary => (lerp_color(PANEL, PANEL_HI, hover), INK),
            Btn::Ghost => (lerp_color(Color32::TRANSPARENT, PANEL, hover), lerp_color(MUTED, INK, hover)),
            Btn::Good => (lerp_color(GOOD, Color32::from_rgb(0x86, 0xdb, 0xa8), hover), ON_GOOD),
            Btn::Bad => (lerp_color(BAD, Color32::from_rgb(0xf0, 0x8a, 0x9c), hover), ON_BAD),
        }
    }
}

/// Size a button needs for `label` at canvas scale (font cond 800 caps).
pub fn button_size(painter: &egui::Painter, c: &Canvas, label: &str, size: f32, pad: Vec2) -> Vec2 {
    let w = text_width(painter, label, c.cond(800, size), 0.08);
    Vec2::new(w, size) + pad * 2.0 * c.s
}

/// Chamfer-A button in `rect`. Hover changes the fill only.
pub fn button(ui: &mut Ui, c: &Canvas, rect: Rect, label: &str, kind: Btn, id: impl std::hash::Hash) -> Response {
    let resp = ui.interact(rect, Id::new(("kit_btn", id)), Sense::click());
    let t = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let (fill, text) = kind.colors(t);
    let p = ui.painter();
    if fill != Color32::TRANSPARENT {
        poly(p, pattern_a(rect, c.v(CHAMFER_MD)), fill, Stroke::NONE);
    }
    let size = (rect.height() * 0.42).min(c.v(theme::TYPE_BUTTON));
    // A leading ✓ / ✕ is drawn as line art (the bundled fonts lack them).
    let (icon, label) = match label.split_once(' ') {
        Some(("✓", rest)) => (Some(true), rest),
        Some(("✕", rest)) => (Some(false), rest),
        _ => (None, label),
    };
    let font = c.cond(800, size / c.s);
    let icon_w = if icon.is_some() { size * 0.8 + c.v(8.0) } else { 0.0 };
    let tw = text_width(p, label, font.clone(), 0.08);
    let x0 = rect.center().x - (icon_w + tw) / 2.0;
    if let Some(check) = icon {
        let s = size * 0.4;
        let ctr = Pos2::new(x0 + size * 0.4, rect.center().y);
        let st = Stroke::new(c.v(2.2), text);
        if check {
            painter_path(p, &[ctr + Vec2::new(-s, 0.0), ctr + Vec2::new(-s * 0.3, s * 0.7), ctr + Vec2::new(s, -s * 0.7)], st);
        } else {
            p.line_segment([ctr + Vec2::new(-s * 0.7, -s * 0.7), ctr + Vec2::new(s * 0.7, s * 0.7)], st);
            p.line_segment([ctr + Vec2::new(-s * 0.7, s * 0.7), ctr + Vec2::new(s * 0.7, -s * 0.7)], st);
        }
    }
    text_tracked(p, Pos2::new(x0 + icon_w, rect.center().y), Align2::LEFT_CENTER, label, font, text, 0.08);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

fn painter_path(p: &egui::Painter, pts: &[Pos2], stroke: Stroke) {
    p.add(Shape::line(pts.to_vec(), stroke));
}

/// Muted tracked key hint next to a button ("ENTER ↵", "C", "ESC").
/// A trailing "↵" is drawn as line art (none of the bundled fonts have it).
pub fn key_hint(painter: &egui::Painter, c: &Canvas, pos: Pos2, align: Align2, text: &str) -> Rect {
    let font = c.cond(700, 13.0);
    let Some(word) = text.strip_suffix('↵') else {
        return text_tracked(painter, pos, align, text, font, MUTED, 0.16);
    };
    let word = word.trim_end();
    let glyph = c.v(11.0);
    let w = text_width(painter, word, font.clone(), 0.16) + c.v(4.0) + glyph;
    let rect = align.anchor_size(pos, Vec2::new(w, c.v(13.0)));
    text_tracked(painter, rect.left_center(), Align2::LEFT_CENTER, word, font, MUTED, 0.16);
    // ↵: a short riser on the right, a stem to the left, and an arrowhead.
    let stroke = Stroke::new(c.v(1.4), MUTED);
    let (r, y) = (rect.right(), rect.center().y + c.v(1.0));
    let l = r - glyph;
    painter.line_segment([Pos2::new(r, y - c.v(5.0)), Pos2::new(r, y)], stroke);
    painter.line_segment([Pos2::new(r, y), Pos2::new(l, y)], stroke);
    painter.line_segment([Pos2::new(l, y), Pos2::new(l + c.v(3.5), y - c.v(3.5))], stroke);
    painter.line_segment([Pos2::new(l, y), Pos2::new(l + c.v(3.5), y + c.v(3.5))], stroke);
    rect
}

/// Fixed top-right "QUIT ESC" button. Returns true on click or Esc.
pub fn quit_button(ui: &mut Ui) -> bool {
    let c = Canvas::new(ui.ctx());
    let rect = c.r(1148.0, 30.0, 92.0, 34.0);
    let resp = ui.interact(rect, Id::new("kit_quit"), Sense::click());
    let t = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let p = ui.painter();
    poly(p, chamfer_pts(rect, 0.0, 0.0, c.v(CHAMFER_SM), 0.0), lerp_color(PANEL, PANEL_HI, t), Stroke::NONE);
    text_tracked(p, rect.center(), Align2::CENTER_CENTER, "QUIT  ESC", c.cond(700, 13.0), lerp_color(MUTED, INK, t), 0.14);
    resp.clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape))
}

// ---------------------------------------------------------------------------
// Segmented control
// ---------------------------------------------------------------------------

/// Square segments with a 4px gap; only the outer ends are chamfered.
/// `pos` is the top-left in screen space. Returns (bounds, clicked index).
pub fn segmented(
    ui: &mut Ui,
    c: &Canvas,
    pos: Pos2,
    labels: &[&str],
    selected: usize,
    id: &str,
) -> (Rect, Option<usize>) {
    let font = c.cond(700, theme::TYPE_SEGMENT);
    let pad = Vec2::new(14.0, 9.0) * c.s;
    let h = c.v(theme::TYPE_SEGMENT) + pad.y * 2.0;
    let gap = c.v(theme::SPACE_SEG);
    let cut = c.v(CHAMFER_SM);
    let mut x = pos.x;
    let mut clicked = None;
    let n = labels.len();
    for (i, label) in labels.iter().enumerate() {
        let w = text_width(ui.painter(), label, font.clone(), 0.08) + pad.x * 2.0;
        let rect = Rect::from_min_size(Pos2::new(x, pos.y), Vec2::new(w, h));
        let resp = ui.interact(rect, Id::new((id, i)), Sense::click());
        let sel = i == selected;
        let hover = ui.ctx().animate_bool_with_time(resp.id, resp.hovered() && !sel, 0.12);
        let fill = if sel { GOLD } else { lerp_color(PANEL, PANEL_HI, hover) };
        let pts = chamfer_pts(rect, if i == 0 { cut } else { 0.0 }, 0.0, if i == n - 1 { cut } else { 0.0 }, 0.0);
        poly(ui.painter(), pts, fill, Stroke::NONE);
        text_tracked(ui.painter(), rect.center(), Align2::CENTER_CENTER, label, font.clone(),
            if sel { ON_GOLD } else { lerp_color(MUTED, INK, hover) }, 0.08);
        if resp.clicked() {
            clicked = Some(i);
        }
        x = rect.right() + gap;
    }
    (Rect::from_min_max(pos, Pos2::new(x - gap, pos.y + h)), clicked)
}

// ---------------------------------------------------------------------------
// Tag — slanted label (ROUND 1, $800, TOSSUP 7)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TagStyle {
    Gold,
    Panel,
}

/// Draws a tag whose left edge is at `pos.x` and top at `pos.y`. Returns its rect.
pub fn tag(painter: &egui::Painter, c: &Canvas, pos: Pos2, text: &str, style: TagStyle, size: f32) -> Rect {
    let font = c.cond(800, size);
    let w = text_width(painter, text, font.clone(), 0.12);
    let h = c.v(size + 20.0);
    let off = c.v(SLANT_TAG);
    let rect = Rect::from_min_size(pos, Vec2::new(w + c.v(28.0) + off, h));
    let (fill, ink) = match style {
        TagStyle::Gold => (GOLD, ON_GOLD),
        TagStyle::Panel => (PANEL, INK),
    };
    poly(painter, tag_pts(rect, off), fill, Stroke::NONE);
    text_tracked(painter, Pos2::new(rect.left() + c.v(14.0), rect.center().y), Align2::LEFT_CENTER, text, font, ink, 0.12);
    rect
}

// ---------------------------------------------------------------------------
// Photo plate + roster cards
// ---------------------------------------------------------------------------

/// Chamfered square showing the team photo, or the default crest (pattern +
/// gold monogram) when none is uploaded. `on_gold` = drawn on an active card.
pub fn photo_plate(painter: &egui::Painter, rect: Rect, name: &str, photo: Option<&TextureHandle>, on_gold: bool) {
    let pts = pattern_a(rect, rect.width() * 0.09);
    match photo {
        Some(tex) => {
            // Textured polygon: map the square image onto the chamfered plate.
            let mut mesh = egui::Mesh::with_texture(tex.id());
            let uv = |p: Pos2| Pos2::new((p.x - rect.left()) / rect.width(), (p.y - rect.top()) / rect.height());
            for p in &pts {
                mesh.vertices.push(egui::epaint::Vertex { pos: *p, uv: uv(*p), color: Color32::WHITE });
            }
            for i in 1..pts.len() as u32 - 1 {
                mesh.add_triangle(0, i, i + 1);
            }
            painter.add(Shape::mesh(mesh));
        }
        None => {
            let (fill, lines) = if on_gold {
                (ON_GOLD, theme::alpha(GOLD, 0.35))
            } else {
                (PANEL_HI, theme::alpha(GOLD, 0.20))
            };
            poly(painter, pts.clone(), fill, Stroke::NONE);
            let kind: Texture = Crest::for_name(name).into();
            let pitch = (rect.width() / 9.0).max(6.0);
            texture(painter, &pts, kind, pitch, lines);
            let mono = monogram(name);
            painter.text(rect.center(), Align2::CENTER_CENTER, mono, theme::cond(800, rect.height() * 0.42), GOLD);
        }
    }
}

/// Photos uploaded per team/player name, stored in egui memory so any screen
/// can draw them. (Upload UI is future work; this is the extension point.)
pub fn team_photo(ctx: &egui::Context, name: &str) -> Option<TextureHandle> {
    ctx.data(|d| d.get_temp::<TextureHandle>(Id::new(("team_photo", name))))
}

#[allow(dead_code)]
pub fn set_team_photo(ctx: &egui::Context, name: &str, tex: TextureHandle) {
    ctx.data_mut(|d| d.insert_temp(Id::new(("team_photo", name)), tex));
}

/// Animated count-up: returns the value to display for `target`, easing from
/// the previous target over 0.6s whenever it changes.
pub fn count_up(ctx: &egui::Context, id: Id, target: i32) -> i32 {
    let now = ctx.input(|i| i.time);
    let (from, to, start) = ctx.data(|d| d.get_temp::<(i32, i32, f64)>(id)).unwrap_or((target, target, now - 1.0));
    let t = ((now - start) / 0.6) as f32;
    let shown = from as f32 + (to - from) as f32 * ease_out(t);
    if to != target {
        ctx.data_mut(|d| d.insert_temp(id, (shown.round() as i32, target, now)));
        ctx.request_repaint();
        return shown.round() as i32;
    }
    if t < 1.0 {
        ctx.request_repaint();
    }
    shown.round() as i32
}

/// One team in a roster row.
pub struct RosterEntry<'a> {
    pub name: &'a str,
    pub score: i32,
    /// Shown after the name in the label ("PICKING", "×2 STREAK").
    pub note: Option<String>,
    /// Right-aligned status ("READY", "LOCKED OUT").
    pub status: Option<&'a str>,
    /// Points change shown next to the score (+20 in good, +0/−n in bad).
    pub delta: Option<i32>,
    /// 4px top border (good/bad) after a reveal.
    pub top: Option<Color32>,
    /// Format the score as money ($1,800).
    pub money: bool,
}

impl<'a> RosterEntry<'a> {
    pub fn new(name: &'a str, score: i32) -> Self {
        Self { name, score, note: None, status: None, delta: None, top: None, money: false }
    }
}

/// Roster cards along the bottom of a screen: the signature element.
/// `area` is in design space (x, width, bottom). The active card grows from
/// 112px to 142px tall, widens and turns gold over 0.2s. `dim_others` drops the
/// non-active cards to 60% (during a buzz).
pub fn roster(ui: &Ui, c: &Canvas, x: f32, w: f32, bottom: f32, entries: &[RosterEntry], active: Option<usize>, dim_others: bool) {
    let n = entries.len();
    if n == 0 {
        return;
    }
    let ctx = ui.ctx();
    let gap = 12.0;
    let ts: Vec<f32> = (0..n)
        .map(|i| ctx.animate_bool_with_time(Id::new(("roster_active", entries[i].name)), active == Some(i), 0.2))
        .collect();
    // Flex weights: the active card is 1.25× as wide.
    let weights: Vec<f32> = ts.iter().map(|t| 1.0 + 0.25 * t).collect();
    let total: f32 = weights.iter().sum();
    let avail = w - gap * (n as f32 - 1.0);
    let painter = ui.painter();
    let mut cx = x;
    for (i, e) in entries.iter().enumerate() {
        let t = ts[i];
        let cw = avail * weights[i] / total;
        let ch = 112.0 + 30.0 * t;
        let rect = c.r(cx, bottom - ch, cw, ch);
        cx += cw + gap;

        let fade = if dim_others && active.is_some() && active != Some(i) { 0.6 } else { 1.0 };
        let fill = lerp_color(PANEL, GOLD, t).gamma_multiply(fade);
        let ink = lerp_color(INK, ON_GOLD, t).gamma_multiply(fade);
        let sub = lerp_color(MUTED, ON_GOLD, t).gamma_multiply(fade);
        let pts = pattern_b(rect, c.v(CHAMFER_LG));
        poly(painter, pts.clone(), fill, Stroke::NONE);
        if let Some(col) = e.top {
            painter.rect_filled(Rect::from_min_size(rect.min, Vec2::new(rect.width(), c.v(4.0))), 0.0, col.gamma_multiply(fade));
        }

        // Photo plate
        let inset = 12.0 + 2.0 * t;
        let plate = ch - inset * 2.0;
        let plate_rect = c.r(0.0, 0.0, plate, plate).translate(rect.min - c.origin + Vec2::splat(c.v(inset)));
        let photo = team_photo(ctx, e.name);
        if fade < 1.0 {
            // Dim by overdrawing the card color at 40% on top of the plate.
            photo_plate(painter, plate_rect, e.name, photo.as_ref(), t > 0.5);
            poly(painter, pattern_a(plate_rect, plate_rect.width() * 0.09), BG.gamma_multiply(0.4), Stroke::NONE);
        } else {
            photo_plate(painter, plate_rect, e.name, photo.as_ref(), t > 0.5);
        }

        // Label: NAME · NOTE
        let tx = plate_rect.right() + c.v(18.0);
        let mut label_text = e.name.to_uppercase();
        if let Some(note) = &e.note {
            label_text = format!("{label_text} · {note}");
        }
        text_tracked(painter, Pos2::new(tx, rect.top() + c.v(14.0 + 8.0 * t)), Align2::LEFT_TOP, &label_text,
            c.cond(700, 14.0), sub, 0.2);

        // Score, counting up
        let shown = count_up(ctx, Id::new(("roster_score", e.name)), e.score);
        let score_text = if e.money { money(shown) } else if shown < 0 { format!("−{}", -shown) } else { shown.to_string() };
        let score_col = if shown < 0 && t < 0.5 { BAD.gamma_multiply(fade) } else { ink };
        let size = 44.0 + (theme::TYPE_SCORE_HI - 44.0) * t;
        let srect = painter.text(Pos2::new(tx, rect.bottom() - c.v(10.0 + 2.0 * t)), Align2::LEFT_BOTTOM, score_text,
            c.cond(800, size), score_col);
        if let Some(d) = e.delta {
            let col = if d > 0 { GOOD } else { BAD };
            let txt = if d >= 0 { format!("+{d}") } else { format!("−{}", -d) };
            painter.text(Pos2::new(srect.right() + c.v(10.0), srect.bottom() - c.v(4.0)), Align2::LEFT_BOTTOM, txt,
                c.cond(800, 22.0), col.gamma_multiply(fade));
        }
        if let Some(status) = e.status {
            text_tracked(painter, Pos2::new(rect.right() - c.v(12.0), rect.center().y), Align2::RIGHT_CENTER, status,
                c.cond(700, 12.0), sub, 0.18);
        }
    }
}

// ---------------------------------------------------------------------------
// Stinger band + skew ticks
// ---------------------------------------------------------------------------

/// A full-width band rotated by `deg` (negative = rising to the right) through
/// `center`, `thick` screen px tall, with optional rotated text on it.
pub fn stinger(painter: &egui::Painter, screen: Rect, center: Pos2, deg: f32, thick: f32, fill: Color32, text: Option<(&str, FontId, Color32)>) {
    let a = deg.to_radians();
    let dir = Vec2::new(a.cos(), a.sin());
    let normal = Vec2::new(-dir.y, dir.x);
    let reach = screen.width().max(screen.height()) * 1.5;
    let half = thick / 2.0;
    let pts = vec![
        center - dir * reach - normal * half,
        center + dir * reach - normal * half,
        center + dir * reach + normal * half,
        center - dir * reach + normal * half,
    ];
    poly(painter, pts, fill, Stroke::NONE);
    if let Some((t, font, col)) = text {
        text_rotated(painter, center, t, font, col, 0.04, a);
    }
}

/// Row of `n` skewed parallelogram ticks; the first `filled` fraction is gold.
pub fn skew_ticks(painter: &egui::Painter, rect: Rect, n: usize, filled: f32, gap: f32) {
    let w = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
    let skew = (rect.height() * 0.45).min(w * 0.5);
    let lit = (filled.clamp(0.0, 1.0) * n as f32).round() as usize;
    for i in 0..n {
        let x = rect.left() + i as f32 * (w + gap);
        let r = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, rect.height()));
        poly(painter, slant_pts(r, skew), if i < lit { GOLD } else { PANEL_HI }, Stroke::NONE);
    }
}

// ---------------------------------------------------------------------------
// Screen-change sweep (the stinger band passing across on every transition)
// ---------------------------------------------------------------------------

const SWEEP_IN: f64 = 0.35;
const SWEEP_HOLD: f64 = 0.12;
const SWEEP_OUT: f64 = 0.30;

/// Start the transition sweep, optionally carrying a label (e.g. "JEOPARDY!").
pub fn start_sweep(ctx: &egui::Context, label: &str) {
    let now = ctx.input(|i| i.time);
    ctx.data_mut(|d| d.insert_temp(Id::new("kit_sweep"), (now, label.to_string())));
}

/// Draw the sweep on top of everything; call once per frame after the screens.
pub fn draw_sweep(ctx: &egui::Context) {
    let Some((start, label)) = ctx.data(|d| d.get_temp::<(f64, String)>(Id::new("kit_sweep"))) else { return };
    let t = ctx.input(|i| i.time) - start;
    let total = SWEEP_IN + SWEEP_HOLD + SWEEP_OUT;
    if t >= total {
        ctx.data_mut(|d| d.remove::<(f64, String)>(Id::new("kit_sweep")));
        return;
    }
    ctx.request_repaint();
    let screen = ctx.screen_rect();
    // Band travels left -> right: off-screen left, covering, off-screen right.
    let travel = screen.width() * 1.6;
    let x = if t < SWEEP_IN {
        -travel / 2.0 * (1.0 - ease_out((t / SWEEP_IN) as f32))
    } else if t < SWEEP_IN + SWEEP_HOLD {
        0.0
    } else {
        let u = ((t - SWEEP_IN - SWEEP_HOLD) / SWEEP_OUT) as f32;
        travel / 2.0 * u * u
    };
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::new("kit_sweep_layer")));
    let c = Canvas::new(ctx);
    let center = screen.center() + Vec2::new(x, 0.0);
    stinger(&painter, screen, center + Vec2::new(0.0, c.v(26.0)), -7.0, c.v(250.0), PANEL_HI, None);
    let text = (!label.is_empty()).then(|| (label.as_str(), c.cond(800, 120.0), BG));
    stinger(&painter, screen, center, -7.0, c.v(220.0), GOLD, text);
}

// ---------------------------------------------------------------------------
// Loading — gold stinger band with ∞, status line, skewed progress
// ---------------------------------------------------------------------------

/// The loading screen. `progress` is 0..1 (None = indeterminate, cycles).
pub fn loading(ui: &mut Ui, status: &str, sub: &str, progress: Option<f32>) {
    backdrop(ui.ctx(), None);
    let c = Canvas::new(ui.ctx());
    let p = ui.painter();
    let center = c.p(640.0, 315.0);
    stinger(p, c.screen, center, -8.0, c.v(155.0), GOLD, None);
    let a = (-8.0f32).to_radians();
    let dir = Vec2::new(a.cos(), a.sin());
    // ∞ glyph then the status, both riding the band.
    text_rotated(p, center + dir * c.v(-407.0), "∞", theme::barlow(600, c.v(110.0)), BG, 0.0, a);
    let galley = tracked_galley(p, status, c.cond(800, 64.0), BG, 0.02);
    let half_w = galley.size().x / 2.0;
    text_rotated(p, center + dir * (c.v(-315.0) + half_w), status, c.cond(800, 64.0), BG, 0.02, a);

    let now = ui.input(|i| i.time) as f32;
    let frac = progress.unwrap_or_else(|| ((now * 1.6) % 7.0).floor() / 6.0);
    skew_ticks(p, c.r(180.0, 600.0, 296.0, 10.0), 6, frac, c.v(6.0));
    label(p, &c, c.p(180.0, 638.0), Align2::LEFT_CENTER, sub, MUTED);
    ui.ctx().request_repaint();
}

// ---------------------------------------------------------------------------
// Round complete
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

/// Single-player summary: the Round Complete screen with YOU as the winner card.
pub fn summary(ui: &mut Ui, kicker: &str, score_text: &str, _sub: &str, stats: &[Stat], enter_start: f64) -> SummaryAction {
    let standings = [Standing { name: "You".into(), score: score_text.to_string(), negative: false }];
    round_complete(ui, kicker, &standings, stats, enter_start)
}

/// One row of final standings (already sorted, winner first).
pub struct Standing {
    pub name: String,
    pub score: String,
    pub negative: bool,
}

/// "07 Summary" in the Roster hi-fi: gold winner card on the left with a
/// rotated WINNERS band, standings + stat tiles on the right, then actions.
pub fn round_complete(ui: &mut Ui, title: &str, standings: &[Standing], stats: &[Stat], enter_start: f64) -> SummaryAction {
    backdrop(ui.ctx(), None);
    let c = Canvas::new(ui.ctx());
    let p = ui.painter().clone();
    let age = since(ui, enter_start, 0.8);
    let mut action = SummaryAction::None;
    let Some(winner) = standings.first() else { return action };
    let solo = standings.len() == 1;

    // -- Winner card ---------------------------------------------------------------
    let slide = (1.0 - ease_out(age / 0.45)) * c.v(-80.0);
    let card = c.r(64.0, 56.0, 560.0, 688.0).translate(Vec2::new(slide, 0.0));
    let card_pts = chamfer_pts(card, c.v(28.0), 0.0, c.v(28.0), 0.0);
    poly(&p, card_pts.clone(), GOLD, Stroke::NONE);
    let photo_r = Rect::from_min_max(card.min + Vec2::new(c.v(24.0), c.v(24.0)), Pos2::new(card.right() - c.v(24.0), card.top() + c.v(400.0)));
    match team_photo(ui.ctx(), &winner.name) {
        Some(tex) => {
            p.image(tex.id(), photo_r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            let pts = vec![photo_r.left_top(), photo_r.right_top(), photo_r.right_bottom(), photo_r.left_bottom()];
            texture(&p, &pts, Texture::Hatch, c.v(12.0), theme::alpha(ON_GOLD, 0.25));
            p.text(photo_r.center(), Align2::CENTER_CENTER, monogram(&winner.name), c.cond(800, 200.0), theme::alpha(ON_GOLD, 0.85));
        }
    }
    // Rotated navy band with the WINNERS stamp.
    let band_c = Pos2::new(card.center().x, card.top() + c.v(462.0));
    let band = {
        let a = (-6.0f32).to_radians();
        let dir = Vec2::new(a.cos(), a.sin());
        let n = Vec2::new(-dir.y, dir.x);
        let (hw, hh) = (card.width() / 2.0 / a.cos(), c.v(32.0));
        vec![band_c - dir * hw - n * hh, band_c + dir * hw - n * hh, band_c + dir * hw + n * hh, band_c - dir * hw + n * hh]
    };
    let band = clip_poly(&band, &card_pts);
    if band.len() >= 3 {
        poly(&p, band, BG, Stroke::NONE);
    }
    let stamp = if solo { "FINAL SCORE" } else { "WINNERS" };
    text_rotated(&p, band_c, stamp, c.cond(800, 44.0), GOLD, 0.16, (-6.0f32).to_radians());
    p.text(Pos2::new(card.left() + c.v(36.0), card.bottom() - c.v(26.0)), Align2::LEFT_BOTTOM, winner.name.to_uppercase(), c.cond(800, 60.0), ON_GOLD);
    p.text(Pos2::new(card.right() - c.v(36.0), card.bottom() - c.v(16.0)), Align2::RIGHT_BOTTOM, &winner.score, c.cond(800, 104.0), ON_GOLD);

    // -- Standings ---------------------------------------------------------------------
    label(&p, &c, c.p(672.0, 66.0), Align2::LEFT_CENTER, &format!("FINAL STANDINGS · {}", title.to_uppercase()), MUTED);
    let mut y = 85.0;
    for (i, st) in standings.iter().enumerate().skip(1).take(3) {
        let row = c.r(672.0, y, 544.0, 92.0);
        poly(&p, pattern_b(row, c.v(CHAMFER_SM)), PANEL, Stroke::NONE);
        p.text(Pos2::new(row.left() + c.v(14.0), row.center().y), Align2::LEFT_CENTER, (i + 1).to_string(), c.cond(800, 40.0), if i == 1 { GOLD } else { MUTED });
        let plate = Rect::from_min_size(Pos2::new(row.left() + c.v(64.0), row.center().y - c.v(32.0)), Vec2::splat(c.v(64.0)));
        photo_plate(&p, plate, &st.name, team_photo(ui.ctx(), &st.name).as_ref(), false);
        p.text(Pos2::new(plate.right() + c.v(16.0), row.center().y), Align2::LEFT_CENTER, st.name.to_uppercase(), c.cond(800, 32.0), INK);
        p.text(Pos2::new(row.right() - c.v(14.0), row.center().y), Align2::RIGHT_CENTER, &st.score, c.cond(800, 44.0), if st.negative { BAD } else { INK });
        y += 102.0;
    }
    if standings.len() > 1 {
        y += 22.0;
        p.line_segment([c.p(672.0, y), c.p(1216.0, y)], Stroke::new(1.0, LINE));
        y += 22.0;
    }

    // -- Stat tiles ----------------------------------------------------------------------
    let n = stats.len().max(1) as f32;
    let tw = (544.0 - 10.0 * (n - 1.0)) / n;
    for (i, s) in stats.iter().enumerate() {
        let r = c.r(672.0 + i as f32 * (tw + 10.0), y, tw, 124.0);
        p.rect_filled(r, 0.0, PANEL);
        p.text(r.min + Vec2::new(c.v(18.0), c.v(16.0)), Align2::LEFT_TOP, &s.value.to_uppercase(), c.cond(800, 46.0), INK);
        label(&p, &c, r.min + Vec2::new(c.v(18.0), c.v(88.0)), Align2::LEFT_CENTER, &s.label.to_uppercase(), MUTED);
    }

    // -- Actions ---------------------------------------------------------------------------
    let by = 618.0;
    if button(ui, &c, c.r(672.0, by, 150.0, 58.0), "REMATCH", Btn::Primary, "rc_again").clicked()
        || ui.input(|i| i.key_pressed(egui::Key::Enter))
    {
        action = SummaryAction::Restart;
    }
    if button(ui, &c, c.r(834.0, by, 110.0, 58.0), "HOME", Btn::Ghost, "rc_home").clicked() {
        action = SummaryAction::Quit;
    }
    key_hint(&p, &c, c.p(958.0, by + 29.0), Align2::LEFT_CENTER, "ENTER ↵");
    action
}

/// Gold-bordered text field (the wager / answer inset). Returns true on Enter.
pub fn answer_field(ui: &mut Ui, field: Rect, buf: &mut String, focus: bool, font_size: f32) -> bool {
    ui.painter().rect_filled(field, 0.0, PANEL);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field.shrink(2.0)).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.visuals_mut().extreme_bg_color = PANEL;
    child.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
    child.visuals_mut().widgets.hovered.bg_stroke = Stroke::NONE;
    child.visuals_mut().selection.stroke = Stroke::NONE;
    let resp = child.add(
        egui::TextEdit::singleline(buf)
            .desired_width(field.width() - 4.0)
            .font(theme::cond(700, font_size))
            .margin(egui::Margin::symmetric(14.0, (field.height() - font_size) / 2.0 - 4.0))
            .frame(false)
            .text_color(INK),
    );
    ui.painter().rect_stroke(field, 0.0, Stroke::new(2.0, if resp.has_focus() { GOLD } else { LINE }));
    if focus {
        resp.request_focus();
    }
    resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}

// ---------------------------------------------------------------------------
// App icon, QR code, dashed outline
// ---------------------------------------------------------------------------

/// The app icon (assets/icon.png) as a texture, decoded once and cached.
pub fn app_icon(ctx: &egui::Context) -> Option<TextureHandle> {
    let id = Id::new("kit_app_icon");
    if let Some(tex) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return Some(tex);
    }
    let img = image::load_from_memory(include_bytes!("../../assets/icon.png")).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    let tex = ctx.load_texture("app_icon", color, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    Some(tex)
}

/// QR modules for `text`: (width, dark flags row-major).
pub fn qr_modules(text: &str) -> Option<(usize, Vec<bool>)> {
    let code = qrcode::QrCode::new(text.as_bytes()).ok()?;
    let w = code.width();
    let dark = (0..w * w).map(|i| code[(i % w, i / w)] == qrcode::Color::Dark).collect();
    Some((w, dark))
}

/// Draw a QR code on an INK plate filling `rect` (with a quiet-zone margin).
pub fn draw_qr(painter: &egui::Painter, rect: Rect, modules: &(usize, Vec<bool>)) {
    let (w, dark) = modules;
    painter.rect_filled(rect, 0.0, INK);
    let inner = rect.shrink(rect.width() * 0.07);
    let px = inner.width() / *w as f32;
    for (i, &d) in dark.iter().enumerate() {
        if d {
            let (x, y) = ((i % w) as f32, (i / w) as f32);
            let m = Rect::from_min_size(inner.min + Vec2::new(x * px, y * px), Vec2::splat(px + 0.3));
            painter.rect_filled(m, 0.0, BG);
        }
    }
}

/// Dashed rectangle outline (the "join" card).
pub fn dashed_rect(painter: &egui::Painter, rect: Rect, dash: f32, gap: f32, stroke: Stroke) {
    let corners = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()];
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        let len = (b - a).length();
        let dir = (b - a) / len;
        let mut s = 0.0;
        while s < len {
            let e = (s + dash).min(len);
            painter.line_segment([a + dir * s, a + dir * e], stroke);
            s += dash + gap;
        }
    }
}

/// Width/height a segmented control will take (to right-align it before drawing).
pub fn segmented_size(painter: &egui::Painter, c: &Canvas, labels: &[&str]) -> Vec2 {
    let font = c.cond(700, theme::TYPE_SEGMENT);
    let pad = c.v(14.0);
    let w: f32 = labels.iter().map(|l| text_width(painter, l, font.clone(), 0.08) + pad * 2.0).sum::<f32>()
        + c.v(theme::SPACE_SEG) * (labels.len() as f32 - 1.0);
    Vec2::new(w, c.v(theme::TYPE_SEGMENT + 18.0))
}

/// Frameless single-line text edit filling `rect` (the caller draws the box).
/// Big condensed numerals for wagers. Returns true when Enter submits.
pub fn answer_field_bare(ui: &mut Ui, rect: Rect, buf: &mut String, focus: bool, font_px: f32, id: &str) -> bool {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.visuals_mut().selection.stroke = Stroke::NONE;
    let resp = child.add(
        egui::TextEdit::singleline(buf)
            .id(Id::new(("field", id)))
            .desired_width(rect.width())
            .font(theme::cond(800, font_px))
            .margin(egui::Margin::symmetric(0.0, ((rect.height() - font_px) / 2.0 - 4.0).max(0.0)))
            .frame(false)
            .text_color(INK),
    );
    if focus {
        resp.request_focus();
    }
    resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}
