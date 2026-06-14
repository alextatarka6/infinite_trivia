use egui::{Align2, Color32, FontId, Id, Pos2, Rect, RichText, Rounding, Sense, Shape, Stroke, Vec2};
use crate::game::state::Screen;
use crate::theme::{ACCENT_SOFT, BLUE_DARK, BLUE_MID, GOLD, INK_DIM, LINE, WHITE};

const CARD_BORDER_DEFAULT: Color32 = Color32::from_rgb(36, 50, 128);

pub struct HomeScreen {
    pub level: u8,
    pub jeopardy_solo: bool,
}

impl Default for HomeScreen {
    fn default() -> Self {
        Self { level: 2, jeopardy_solo: true }
    }
}

impl HomeScreen {
    fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
        Color32::from_rgb(
            (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
            (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
            (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        )
    }

    fn draw_board_glyph(painter: &egui::Painter, center: Pos2, s: f32) {
        let sq = 6.0 * s;
        let step = sq + 3.0 * s;
        let offset = step;
        for row in 0..3i32 {
            for col in 0..3i32 {
                let rect = Rect::from_min_size(
                    Pos2::new(
                        center.x - offset + col as f32 * step,
                        center.y - offset + row as f32 * step,
                    ),
                    Vec2::splat(sq),
                );
                painter.rect_filled(rect, Rounding::same(1.5 * s), GOLD);
            }
        }
    }

    fn draw_target_glyph(painter: &egui::Painter, center: Pos2, s: f32) {
        painter.circle_stroke(center, 12.0 * s, Stroke::new(2.5 * s, GOLD));
        painter.circle_filled(center, 4.5 * s, GOLD);
    }

    /// Radial glow backdrop: lighter blue at upper-center fading to near-black edges.
    /// The glow gently "breathes" — its radius and brightness pulse over time.
    fn paint_backdrop(ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());

        // Near-black base — keeps corners and lower edge deep
        painter.rect_filled(screen, Rounding::ZERO, Color32::from_rgb(5, 7, 22));

        // Breathing: slow sine over time, normalized to 0..1.
        let time = ctx.input(|i| i.time) as f32;
        let breath = 0.5 + 0.5 * (time * 0.55).sin(); // ~11s period
        let radius_scale = 0.97 + 0.06 * breath; // 0.97 .. 1.03
        let bright = 0.88 + 0.12 * breath; // 0.88 .. 1.00
        ctx.request_repaint(); // keep the animation alive

        // Radial glow centered high, behind the wordmark. Wider than tall so the
        // lower half of the screen falls off into the dark base.
        let center = Pos2::new(screen.center().x, screen.top() + screen.height() * 0.24);
        let radius = screen.width().max(screen.height()) * 0.60 * radius_scale;
        let outer = Color32::TRANSPARENT;

        let dim = |c: Color32, f: f32| {
            Color32::from_rgb(
                (c.r() as f32 * f) as u8,
                (c.g() as f32 * f) as u8,
                (c.b() as f32 * f) as u8,
            )
        };

        // Two stops (bright core + broad halo) for a smoother, more natural falloff.
        let glow = |painter: &egui::Painter, inner: Color32, rx: f32, ry: f32| {
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(center, inner);
            let segments = 72u32;
            for i in 0..=segments {
                let a = i as f32 / segments as f32 * std::f32::consts::TAU;
                let p = Pos2::new(center.x + a.cos() * rx, center.y + a.sin() * ry);
                mesh.colored_vertex(p, outer);
            }
            for i in 0..segments {
                mesh.add_triangle(0, 1 + i, 1 + (i + 1) % (segments + 1));
            }
            painter.add(Shape::mesh(mesh));
        };

        // Broad halo
        glow(&painter, dim(Color32::from_rgb(30, 41, 100), bright), radius * 1.20, radius * 0.78);
        // Brighter tight core
        glow(&painter, dim(Color32::from_rgb(42, 56, 130), bright), radius * 0.62, radius * 0.46);
    }

    fn draw_play_glyph(painter: &egui::Painter, center: Pos2, s: f32) {
        let pts = vec![
            Pos2::new(center.x - 9.0 * s, center.y - 13.0 * s),
            Pos2::new(center.x - 9.0 * s, center.y + 13.0 * s),
            Pos2::new(center.x + 12.0 * s, center.y),
        ];
        painter.add(Shape::convex_polygon(pts, GOLD, Stroke::NONE));
    }

    // Reusable score chip for other screens to call
    pub fn score_chip(painter: &egui::Painter, rect: Rect, label: &str, value: &str) {
        painter.rect_filled(rect, Rounding::same(999.0), BLUE_MID);
        painter.rect_stroke(rect, Rounding::same(999.0), Stroke::new(1.0, CARD_BORDER_DEFAULT));
        let mid = rect.center();
        painter.text(
            Pos2::new(mid.x - 6.0, mid.y),
            Align2::RIGHT_CENTER,
            label,
            crate::theme::mono_font(10.5),
            INK_DIM,
        );
        painter.text(
            Pos2::new(mid.x - 2.0, mid.y),
            Align2::LEFT_CENTER,
            value,
            FontId::proportional(14.0),
            WHITE,
        );
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<(Screen, u8)> {
        let mut result: Option<(Screen, u8)> = None;
        let mut new_level: Option<u8> = None;
        let current_level = self.level;
        let avail = ui.available_size();

        Self::paint_backdrop(ui.ctx());

        // -- Reference-matched proportions (driven off available size) -----------
        let w = avail.x;
        let h = avail.y;

        let card_w = w * 0.1965;
        let card_h = card_w * 0.945;
        let gap = w * 0.0225;
        let icon_s = card_w / 200.0;

        ui.add_space(h * 0.115);

        ui.vertical_centered(|ui| {
            // Wordmark
            ui.label(
                RichText::new("I N F I N I T E")
                    .font(crate::theme::spectral_medium(w * 0.0165))
                    .color(WHITE.linear_multiply(0.85)),
            );
            ui.add_space(-h * 0.008);
            ui.label(
                RichText::new("TRIVIA")
                    .font(crate::theme::spectral(w * 0.094))
                    .color(GOLD),
            );

            ui.add_space(h * 0.052);

            // Cards row
            let total_w = 3.0 * card_w + 2.0 * gap;

            let cards: &[(&str, &str, Screen)] = &[
                ("board", "Jeopardy!", Screen::JeopardySetup),
                ("target", "General Trivia", Screen::Trivia),
                ("play", "Quizbowl", Screen::Quizbowl),
            ];

            ui.horizontal(|ui| {
                ui.add_space((avail.x - total_w) / 2.0);

                for (i, (glyph, name, screen)) in cards.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(gap);
                    }

                    let (rect, resp) = ui.allocate_exact_size(
                        Vec2::new(card_w, card_h),
                        Sense::click(),
                    );

                    let hover_t = ui.ctx().animate_bool(
                        Id::new(("card_hover", i)),
                        resp.hovered(),
                    );

                    // Visual lift on hover; gold border only appears when highlighted.
                    let draw_rect = rect.translate(egui::vec2(0.0, -hover_t * 5.0));
                    let border_color = Self::lerp_color(CARD_BORDER_DEFAULT, GOLD, hover_t);
                    let border_w = egui::lerp(1.0..=2.0, hover_t);

                    // Card background + border
                    let radius = card_w * 0.072;
                    ui.painter().rect_filled(draw_rect, Rounding::same(radius), BLUE_MID);
                    ui.painter().rect_stroke(
                        draw_rect,
                        Rounding::same(radius),
                        Stroke::new(border_w, border_color),
                    );

                    // Icon
                    let glyph_center =
                        Pos2::new(draw_rect.center().x, draw_rect.top() + card_h * 0.31);
                    match *glyph {
                        "board" => Self::draw_board_glyph(ui.painter(), glyph_center, icon_s),
                        "target" => Self::draw_target_glyph(ui.painter(), glyph_center, icon_s),
                        _ => Self::draw_play_glyph(ui.painter(), glyph_center, icon_s),
                    }

                    // Name (Spectral serif)
                    ui.painter().text(
                        Pos2::new(draw_rect.center().x, draw_rect.top() + card_h * 0.60),
                        Align2::CENTER_CENTER,
                        *name,
                        crate::theme::spectral(card_w * 0.102),
                        WHITE,
                    );

                    // Solo / Teams toggle (Jeopardy only)
                    let mut toggle_clicked = false;
                    if i == 0 {
                        let pill_cx = draw_rect.center().x;
                        let pill_cy = draw_rect.top() + card_h * 0.81;
                        let solo_w = card_w * 0.225;
                        let teams_w = card_w * 0.275;
                        let inner_h = card_h * 0.095;
                        let pad = card_w * 0.014;

                        let outer_rect = Rect::from_center_size(
                            Pos2::new(pill_cx, pill_cy),
                            Vec2::new(solo_w + teams_w + pad * 3.0, inner_h + pad * 2.0),
                        );
                        ui.painter().rect_filled(outer_rect, Rounding::same(999.0), BLUE_DARK);

                        let solo_rect = Rect::from_center_size(
                            Pos2::new(pill_cx - (teams_w / 2.0 + pad / 2.0), pill_cy),
                            Vec2::new(solo_w, inner_h),
                        );
                        let teams_rect = Rect::from_center_size(
                            Pos2::new(pill_cx + (solo_w / 2.0 + pad / 2.0), pill_cy),
                            Vec2::new(teams_w, inner_h),
                        );

                        if self.jeopardy_solo {
                            ui.painter().rect_filled(solo_rect, Rounding::same(999.0), GOLD);
                        } else {
                            ui.painter().rect_filled(teams_rect, Rounding::same(999.0), GOLD);
                        }

                        let pill_font = crate::theme::plex_medium(card_w * 0.058);
                        ui.painter().text(
                            solo_rect.center(),
                            Align2::CENTER_CENTER,
                            "Solo",
                            pill_font.clone(),
                            if self.jeopardy_solo {
                                Color32::from_rgb(30, 20, 5)
                            } else {
                                INK_DIM
                            },
                        );
                        ui.painter().text(
                            teams_rect.center(),
                            Align2::CENTER_CENTER,
                            "Teams",
                            pill_font,
                            if !self.jeopardy_solo {
                                Color32::from_rgb(30, 20, 5)
                            } else {
                                INK_DIM
                            },
                        );

                        let solo_resp =
                            ui.interact(solo_rect, Id::new("solo_toggle"), Sense::click());
                        let teams_resp =
                            ui.interact(teams_rect, Id::new("teams_toggle"), Sense::click());
                        if solo_resp.clicked() {
                            self.jeopardy_solo = true;
                            toggle_clicked = true;
                        }
                        if teams_resp.clicked() {
                            self.jeopardy_solo = false;
                            toggle_clicked = true;
                        }
                    }

                    if resp.clicked() && !toggle_clicked {
                        result = Some((screen.clone(), current_level));
                    }
                }
            });

            ui.add_space(h * 0.058);

            // Difficulty label
            let level_names = ["EASY", "MEDIUM", "HARD"];
            ui.label(
                RichText::new(format!(
                    "DIFFICULTY  ·  {}",
                    level_names[current_level as usize - 1]
                ))
                .font(crate::theme::mono_medium(w * 0.0098))
                .color(INK_DIM),
            );

            ui.add_space(h * 0.018);

            // Difficulty dots + rail
            let dot_r = w * 0.0055;
            let dot_r_sel = dot_r + w * 0.002;
            let dot_slot = dot_r_sel * 2.0;
            let rail_gap = w * 0.047;
            let row_w = 3.0 * dot_slot + 2.0 * rail_gap;
            let left_margin = (avail.x - row_w) / 2.0 - 8.0;

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
                        ui.painter().circle_stroke(c, r + 3.0, Stroke::new(1.0, ACCENT_SOFT));
                    }
                    let border_c = if n <= current_level { GOLD } else { LINE };
                    ui.painter().circle_stroke(c, r, Stroke::new(1.5, border_c));

                    if dot_resp.clicked() {
                        new_level = Some(n);
                    }

                    if n < 3 {
                        let (rail_rect, _) =
                            ui.allocate_exact_size(Vec2::new(rail_gap, 4.0), Sense::hover());
                        let y = rail_rect.center().y;
                        ui.painter().line_segment(
                            [Pos2::new(rail_rect.min.x, y), Pos2::new(rail_rect.max.x, y)],
                            Stroke::new(2.0, LINE),
                        );
                        if current_level > n {
                            ui.painter().line_segment(
                                [Pos2::new(rail_rect.min.x, y), Pos2::new(rail_rect.max.x, y)],
                                Stroke::new(2.0, GOLD),
                            );
                        }
                    }
                }
            });

            ui.add_space(h * 0.03);

            ui.label(
                RichText::new("pick a mode to start")
                    .font(crate::theme::mono_font(w * 0.0098))
                    .color(INK_DIM),
            );
        });

        if let Some(lv) = new_level {
            self.level = lv;
        }

        result
    }
}
