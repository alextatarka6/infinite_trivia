use egui::{Color32, FontFamily, FontId, Rounding, Stroke, Style, Visuals};

// -- Palette ------------------------------------------------------------------
pub const BLUE_BG: Color32 = Color32::from_rgb(10, 17, 64);       // #0a1140  bg
pub const BLUE_MID: Color32 = Color32::from_rgb(13, 22, 87);      // #0d1657  bg-2 / panels
pub const BLUE_DARK: Color32 = Color32::from_rgb(26, 47, 150);    // #1a2f96  tile fill
pub const GOLD: Color32 = Color32::from_rgb(244, 182, 63);        // #f4b63f  accent
pub const GOLD_HOVER: Color32 = Color32::from_rgb(255, 216, 132); // #ffd884  accent-2
pub const WHITE: Color32 = Color32::from_rgb(247, 245, 238);      // #f7f5ee  ink
pub const INK_DIM: Color32 = Color32::from_rgb(170, 179, 228);    // #aab3e4  ink-dim
pub const GREEN_CORRECT: Color32 = Color32::from_rgb(94, 201, 138);  // #5ec98a
pub const RED_WRONG: Color32 = Color32::from_rgb(232, 98, 122);   // #e8627a
// Semi-transparent helpers (stored premultiplied: pre = round(val * alpha / 255))
pub const LINE: Color32 = Color32::from_rgba_premultiplied(31, 32, 41, 46);       // aab3e4 @ 18%
pub const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(74, 55, 19, 77); // f4b63f @ 30%
pub const CORRECT_BG: Color32 = Color32::from_rgba_premultiplied(15, 32, 22, 41);  // 5ec98a @ 16%
pub const WRONG_BG: Color32 = Color32::from_rgba_premultiplied(33, 14, 17, 36);    // e8627a @ 14%

// -- Global style -------------------------------------------------------------
pub fn apply(ctx: &egui::Context) {
    let mut style = Style::default();
    let mut visuals = Visuals::dark();

    visuals.window_fill = BLUE_BG;
    visuals.panel_fill = BLUE_BG;
    visuals.extreme_bg_color = BLUE_DARK;
    visuals.faint_bg_color = BLUE_MID;

    visuals.widgets.noninteractive.bg_fill = BLUE_MID;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, WHITE);

    visuals.widgets.inactive.bg_fill = BLUE_DARK;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, GOLD);
    visuals.widgets.inactive.rounding = Rounding::same(999.0);

    visuals.widgets.hovered.bg_fill = GOLD;
    visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, Color32::from_rgb(26, 19, 4));
    visuals.widgets.hovered.rounding = Rounding::same(999.0);

    visuals.widgets.active.bg_fill = GOLD_HOVER;
    visuals.widgets.active.fg_stroke = Stroke::new(2.0, Color32::from_rgb(26, 19, 4));
    visuals.widgets.active.rounding = Rounding::same(999.0);

    visuals.selection.bg_fill = ACCENT_SOFT;
    visuals.selection.stroke = Stroke::new(1.0, GOLD);
    visuals.override_text_color = Some(WHITE);

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(18.0, 11.0);

    ctx.set_style(style);
}

// -- Font helpers -------------------------------------------------------------
pub fn heading_font() -> FontId { FontId::proportional(30.0) }
pub fn subheading_font() -> FontId { FontId::proportional(20.0) }
pub fn body_font() -> FontId { FontId::proportional(16.0) }
pub fn dollar_font() -> FontId { FontId::proportional(24.0) }
pub fn mono_font(size: f32) -> FontId { FontId::new(size, FontFamily::Monospace) }
