use egui::{Align2, Id, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use crate::screens::kit;
use crate::theme::{self, ACCENT_SOFT, BLUE_BG, GOLD, INK_DIM, LINE, ON_ACCENT, WHITE};

#[derive(Clone, PartialEq)]
pub enum JeopardyMode {
    Solo,
    Teams,
}

pub struct JeopardyConfig {
    pub mode: JeopardyMode,
    pub team_count: u8,
    pub team_names: Vec<String>,
}

pub enum SetupAction {
    Start(JeopardyConfig),
    Back,
}

pub struct JeopardySetupScreen {
    mode: JeopardyMode,
    team_count: u8,
    team_names: [String; 4],
    enter_start: f64,
}

impl Default for JeopardySetupScreen {
    fn default() -> Self {
        Self {
            mode: JeopardyMode::Solo,
            team_count: 2,
            team_names: [
                "Team 1".to_string(),
                "Team 2".to_string(),
                "Team 3".to_string(),
                "Team 4".to_string(),
            ],
            enter_start: -1.0,
        }
    }
}

impl JeopardySetupScreen {
    /// Dev helper: start the screen pre-set to Teams mode (for the dev gallery).
    pub fn dev_teams() -> Self {
        let mut s = Self::default();
        s.mode = JeopardyMode::Teams;
        s.team_count = 3;
        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<SetupAction> {
        kit::ambient(ui.ctx());
        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        let ey = kit::enter_offset(ui, self.enter_start);

        let avail = ui.available_size();
        let cx = avail.x / 2.0;
        let painter = ui.painter().clone();
        let teams = self.mode == JeopardyMode::Teams;
        let mut action = None;

        let content_h = if teams { 470.0 } else { 300.0 };
        let mut y = ((avail.y - content_h) / 2.0).max(30.0) + ey;

        // -- Title ------------------------------------------------------------
        painter.text(Pos2::new(cx, y + 19.0), Align2::CENTER_CENTER, "JEOPARDY!", theme::spectral(38.0), GOLD);
        y += 42.0;
        painter.text(Pos2::new(cx, y), Align2::CENTER_CENTER, "GAME SETUP", theme::mono(12.0), INK_DIM);
        y += 36.0;

        // -- Mode segmented toggle (.it-seg) ----------------------------------
        painter.text(Pos2::new(cx, y), Align2::CENTER_CENTER, "MODE", theme::mono(10.0), INK_DIM);
        y += 22.0;
        let sel = if teams { 1 } else { 0 };
        if let Some(i) = seg_toggle(ui, &painter, cx, y + 17.0, ["SOLO", "TEAMS"], sel) {
            self.mode = if i == 0 { JeopardyMode::Solo } else { JeopardyMode::Teams };
        }
        y += 34.0 + 4.0;

        if teams {
            // -- Number of teams stepper (.it-stepper) ------------------------
            y += 22.0;
            painter.text(Pos2::new(cx, y), Align2::CENTER_CENTER, "NUMBER OF TEAMS", theme::mono(10.0), INK_DIM);
            y += 24.0;
            let delta = stepper(ui, &painter, cx, y + 16.0, &self.team_count.to_string());
            self.team_count = (self.team_count as i32 + delta).clamp(2, 4) as u8;
            y += 32.0 + 18.0;

            // -- Team names ---------------------------------------------------
            painter.text(Pos2::new(cx, y), Align2::CENTER_CENTER, "TEAM NAMES", theme::mono(10.0), INK_DIM);
            y += 20.0;
            for i in 0..self.team_count as usize {
                let field_w = 280.0;
                let num_w = 18.0;
                let gap = 10.0;
                let row_w = num_w + gap + field_w;
                let left = cx - row_w / 2.0;
                painter.text(
                    Pos2::new(left, y + 21.0),
                    Align2::LEFT_CENTER,
                    format!("{}.", i + 1),
                    theme::mono(12.0),
                    INK_DIM,
                );
                let field = Rect::from_min_size(Pos2::new(left + num_w + gap, y), Vec2::new(field_w, 42.0));
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(field)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                child.add(
                    egui::TextEdit::singleline(&mut self.team_names[i])
                        .desired_width(field_w)
                        .font(theme::mono(15.0))
                        .margin(egui::Margin::symmetric(14.0, 11.0))
                        .text_color(WHITE),
                );
                y += 50.0;
            }
        }

        // -- Start (primary) + Back (ghost) -----------------------------------
        y += 28.0;
        let pw = kit_measure(ui, "Start Game") + 44.0;
        let gw = kit_measure(ui, "Back") + 44.0;
        let lp = cx - (pw + 12.0 + gw) / 2.0;
        let mut pc = child(ui, Rect::from_min_size(Pos2::new(lp, y), Vec2::new(pw, 42.0)));
        if kit::primary_button(&mut pc, "Start Game").clicked() {
            let team_names: Vec<String> = (0..self.team_count as usize)
                .map(|i| {
                    let s = self.team_names[i].trim().to_string();
                    if s.is_empty() { format!("Team {}", i + 1) } else { s }
                })
                .collect();
            action = Some(SetupAction::Start(JeopardyConfig {
                mode: self.mode.clone(),
                team_count: self.team_count,
                team_names,
            }));
        }
        let mut gc = child(ui, Rect::from_min_size(Pos2::new(lp + pw + 12.0, y), Vec2::new(gw, 42.0)));
        if kit::ghost_button(&mut gc, "Back").clicked() {
            action = Some(SetupAction::Back);
        }

        action
    }
}

/// `.it-seg` two-option segmented pill. Returns the clicked index, if any.
fn seg_toggle(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    cx: f32,
    cy: f32,
    labels: [&str; 2],
    selected: usize,
) -> Option<usize> {
    let font = theme::plex_semibold(12.0);
    let seg_h = 28.0;
    let pad = 3.0;
    let w: Vec<f32> = labels
        .iter()
        .map(|l| painter.layout_no_wrap((*l).into(), font.clone(), WHITE).size().x + 32.0)
        .collect();
    let total = w[0] + w[1] + pad * 2.0;
    let left = cx - total / 2.0;
    let container = Rect::from_min_size(Pos2::new(left, cy - (seg_h + pad * 2.0) / 2.0), Vec2::new(total, seg_h + pad * 2.0));
    painter.rect_filled(container, Rounding::same(999.0), BLUE_BG);
    painter.rect_stroke(container, Rounding::same(999.0), Stroke::new(1.0, LINE));

    let mut clicked = None;
    let mut x = left + pad;
    for (i, label) in labels.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(x, container.top() + pad), Vec2::new(w[i], seg_h));
        if i == selected {
            painter.rect_filled(r, Rounding::same(999.0), GOLD);
        }
        painter.text(
            r.center(),
            Align2::CENTER_CENTER,
            *label,
            font.clone(),
            if i == selected { ON_ACCENT } else { INK_DIM },
        );
        if ui.interact(r, Id::new(("seg", i)), Sense::click()).clicked() {
            clicked = Some(i);
        }
        x += w[i];
    }
    clicked
}

/// `.it-stepper` ◀ N ▶. Returns -1, 0, or +1.
fn stepper(ui: &mut egui::Ui, painter: &egui::Painter, cx: f32, cy: f32, value: &str) -> i32 {
    let btn = 26.0;
    let pad = 3.0;
    let val_w = 58.0;
    let total = pad + btn + val_w + btn + pad;
    let left = cx - total / 2.0;
    let h = btn + pad * 2.0;
    let container = Rect::from_min_size(Pos2::new(left, cy - h / 2.0), Vec2::new(total, h));
    painter.rect_filled(container, Rounding::same(999.0), BLUE_BG);
    painter.rect_stroke(container, Rounding::same(999.0), Stroke::new(1.0, LINE));

    let mut delta = 0;
    for (i, glyph) in ["◀", "▶"].iter().enumerate() {
        let c = if i == 0 {
            Pos2::new(left + pad + btn / 2.0, container.center().y)
        } else {
            Pos2::new(container.right() - pad - btn / 2.0, container.center().y)
        };
        let r = Rect::from_center_size(c, Vec2::splat(btn));
        let resp = ui.interact(r, Id::new(("step", i)), Sense::click());
        if resp.hovered() {
            painter.circle_filled(c, btn / 2.0, ACCENT_SOFT);
        }
        painter.text(c, Align2::CENTER_CENTER, *glyph, theme::plex(15.0), GOLD);
        if resp.clicked() {
            delta = if i == 0 { -1 } else { 1 };
        }
    }
    painter.text(
        Pos2::new(left + pad + btn + val_w / 2.0, container.center().y),
        Align2::CENTER_CENTER,
        value,
        theme::mono(13.0),
        WHITE,
    );
    delta
}

fn kit_measure(ui: &egui::Ui, label: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(label.to_string(), theme::plex_semibold(14.0), WHITE)
        .size()
        .x
}

fn child(ui: &mut egui::Ui, rect: Rect) -> egui::Ui {
    ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    )
}
