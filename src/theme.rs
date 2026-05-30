use egui::{Color32, FontId, Rounding, Stroke, Style, Visuals};

pub const BLUE_DARK: Color32 = Color32::from_rgb(6, 12, 233);
pub const BLUE_BG: Color32 = Color32::from_rgb(3, 7, 80);
pub const BLUE_MID: Color32 = Color32::from_rgb(10, 20, 150);
pub const GOLD: Color32 = Color32::from_rgb(245, 166, 35);
pub const GOLD_HOVER: Color32 = Color32::from_rgb(255, 195, 80);
pub const WHITE: Color32 = Color32::WHITE;
pub const RED_WRONG: Color32 = Color32::from_rgb(220, 50, 50);
pub const GREEN_CORRECT: Color32 = Color32::from_rgb(50, 200, 80);

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
    visuals.widgets.inactive.rounding = Rounding::same(6.0);

    visuals.widgets.hovered.bg_fill = GOLD;
    visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, BLUE_BG);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);

    visuals.widgets.active.bg_fill = GOLD_HOVER;
    visuals.widgets.active.fg_stroke = Stroke::new(2.0, BLUE_BG);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    visuals.selection.bg_fill = GOLD.linear_multiply(0.4);
    visuals.selection.stroke = Stroke::new(1.0, GOLD);

    visuals.override_text_color = Some(WHITE);

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(16.0, 10.0);

    ctx.set_style(style);
}

pub fn heading_font() -> FontId {
    FontId::proportional(28.0)
}

pub fn subheading_font() -> FontId {
    FontId::proportional(18.0)
}

pub fn body_font() -> FontId {
    FontId::proportional(16.0)
}

pub fn dollar_font() -> FontId {
    FontId::proportional(22.0)
}
