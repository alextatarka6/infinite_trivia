use egui::{FontFamily, FontId, Rounding, Stroke, Style, Visuals};

// -- Design tokens ------------------------------------------------------------
// Color / radius / spacing / type constants are generated at build time from
// `design/tokens.toml` (see build.rs). Edit that file, not these names.
// This brings BLUE_BG, BLUE_MID, BLUE_DARK, GOLD, GOLD_HOVER, WHITE, INK_DIM,
// GREEN_CORRECT, RED_WRONG, ON_ACCENT, BODY_BG, APP_TOP/MID/BOTTOM, TILE_*,
// LINE, ACCENT_SOFT, CORRECT_BG, WRONG_BG, RADIUS_*, SPACE_*, TYPE_* into scope.
// Some tokens aren't consumed by every screen yet, hence allow(dead_code).
mod tokens {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/design_tokens.rs"));
}
pub use tokens::*;

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
    visuals.widgets.inactive.rounding = Rounding::same(RADIUS_PILL);

    visuals.widgets.hovered.bg_fill = GOLD;
    visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, ON_ACCENT);
    visuals.widgets.hovered.rounding = Rounding::same(RADIUS_PILL);

    visuals.widgets.active.bg_fill = GOLD_HOVER;
    visuals.widgets.active.fg_stroke = Stroke::new(2.0, ON_ACCENT);
    visuals.widgets.active.rounding = Rounding::same(RADIUS_PILL);

    visuals.selection.bg_fill = ACCENT_SOFT;
    visuals.selection.stroke = Stroke::new(1.0, GOLD);
    visuals.override_text_color = Some(WHITE);

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(SPACE_ITEM, SPACE_ITEM);
    style.spacing.button_padding = egui::vec2(SPACE_BUTTON_X, SPACE_BUTTON_Y);

    ctx.set_style(style);
}

// -- Font helpers -------------------------------------------------------------

// Spectral — serif display face
pub fn spectral(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("spectral".into()))
}
pub fn spectral_medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("spectral-medium".into()))
}
pub fn spectral_regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("spectral-regular".into()))
}
pub fn spectral_italic(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("spectral-italic".into()))
}

// IBM Plex Sans — UI / body
pub fn plex(size: f32) -> FontId {
    FontId::proportional(size)
}
pub fn plex_medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("plex-medium".into()))
}
pub fn plex_semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("plex-semibold".into()))
}

// IBM Plex Mono — numerals, dollar values, category tags, kicker labels
pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}
pub fn mono_medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono-medium".into()))
}
pub fn mono_semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono-semibold".into()))
}

// Role helpers — sizes come straight from the type-scale tokens.
pub fn heading_font() -> FontId {
    spectral(TYPE_HEADING)
}
pub fn subheading_font() -> FontId {
    plex_medium(TYPE_SUBHEADING)
}
pub fn body_font() -> FontId {
    plex(TYPE_BODY)
}
pub fn dollar_font() -> FontId {
    mono_semibold(TYPE_DOLLAR)
}
pub fn mono_font(size: f32) -> FontId {
    mono(size)
}
