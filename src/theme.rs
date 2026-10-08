use egui::{Color32, FontFamily, FontId, Rounding, Stroke, Style, Visuals};

// -- Design tokens ------------------------------------------------------------
// Color / shape / texture / spacing / type constants are generated at build
// time from `theme/tokens.toml` (see build.rs). Edit that file, not these names.
// This brings BG, PANEL, PANEL_HI, GOLD, GOLD_HI, ON_GOLD, INK, MUTED, GOOD,
// ON_GOOD, BAD, ON_BAD, LINE, CANVAS_*, CHAMFER_*, SLANT_*, TEXTURE_*, SPACE_*
// and TYPE_* into scope. Some aren't consumed yet, hence allow(dead_code).
mod tokens {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/design_tokens.rs"));
}
pub use tokens::*;

/// `c` at `alpha` (0..1), premultiplied — for the textures and hairlines.
pub fn alpha(c: Color32, alpha: f32) -> Color32 {
    c.gamma_multiply(alpha)
}

// -- Global style -------------------------------------------------------------
// Flat, square-cornered widgets. Hover changes the fill only (no lift/glow).
pub fn apply(ctx: &egui::Context) {
    let mut style = Style::default();
    let mut visuals = Visuals::dark();

    visuals.window_fill = BG;
    visuals.panel_fill = BG;
    visuals.extreme_bg_color = PANEL;
    visuals.faint_bg_color = PANEL;
    visuals.window_rounding = Rounding::ZERO;
    visuals.menu_rounding = Rounding::ZERO;

    for w in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        w.rounding = Rounding::ZERO;
        w.expansion = 0.0;
    }
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, INK);
    visuals.widgets.inactive.bg_fill = PANEL;
    visuals.widgets.inactive.weak_bg_fill = PANEL;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, INK);
    visuals.widgets.hovered.bg_fill = PANEL_HI;
    visuals.widgets.hovered.weak_bg_fill = PANEL_HI;
    visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, INK);
    visuals.widgets.active.bg_fill = GOLD;
    visuals.widgets.active.weak_bg_fill = GOLD;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, ON_GOLD);

    visuals.selection.bg_fill = alpha(GOLD, 0.35);
    visuals.selection.stroke = Stroke::new(2.0, GOLD);
    visuals.text_cursor.stroke = Stroke::new(3.0, GOLD);
    visuals.override_text_color = Some(INK);

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(SPACE_ITEM, SPACE_ITEM);

    ctx.set_style(style);
}

// -- Fonts ----------------------------------------------------------------------
// Barlow Condensed: headlines, numbers, scores, labels, buttons (usually caps).
// Barlow: body, descriptions, tossup reading text. Registered in App::new.

fn family(name: &str) -> FontFamily {
    FontFamily::Name(name.into())
}

/// Barlow Condensed at `weight` (500, 600, 700 or 800).
pub fn cond(weight: u16, size: f32) -> FontId {
    let name = match weight {
        0..=550 => "cond-500",
        551..=650 => "cond-600",
        651..=750 => "cond-700",
        _ => "cond-800",
    };
    FontId::new(size, family(name))
}

/// Barlow at `weight` (400, 500 or 600).
pub fn barlow(weight: u16, size: f32) -> FontId {
    let name = match weight {
        0..=450 => "barlow-400",
        451..=550 => "barlow-500",
        _ => "barlow-600",
    };
    FontId::new(size, family(name))
}
