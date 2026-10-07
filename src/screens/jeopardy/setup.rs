use egui::{Align2, Pos2, Rect, Vec2};
use crate::screens::kit;
use crate::theme::{self, GOLD, INK_DIM, WHITE};

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
    enter_start: f64,
}

impl Default for JeopardySetupScreen {
    fn default() -> Self {
        Self { enter_start: -1.0 }
    }
}

impl JeopardySetupScreen {
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
        let mut action = None;

        let content_h = 220.0;
        let mut y = ((avail.y - content_h) / 2.0).max(30.0) + ey;

        // -- Title ------------------------------------------------------------
        painter.text(Pos2::new(cx, y + 19.0), Align2::CENTER_CENTER, "JEOPARDY!", theme::spectral(38.0), GOLD);
        y += 42.0;
        painter.text(Pos2::new(cx, y), Align2::CENTER_CENTER, "SOLO", theme::mono(12.0), INK_DIM);
        y += 60.0;

        // -- Start (primary) + Back (ghost) -----------------------------------
        let pw = kit_measure(ui, "Start Game") + 44.0;
        let gw = kit_measure(ui, "Back") + 44.0;
        let lp = cx - (pw + 12.0 + gw) / 2.0;
        let mut pc = child(ui, Rect::from_min_size(Pos2::new(lp, y), Vec2::new(pw, 42.0)));
        if kit::primary_button(&mut pc, "Start Game").clicked() {
            action = Some(SetupAction::Start(JeopardyConfig {
                mode: JeopardyMode::Solo,
                team_count: 1,
                team_names: vec!["You".to_string()],
            }));
        }
        let mut gc = child(ui, Rect::from_min_size(Pos2::new(lp + pw + 12.0, y), Vec2::new(gw, 42.0)));
        if kit::ghost_button(&mut gc, "Back").clicked() {
            action = Some(SetupAction::Back);
        }

        action
    }
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
