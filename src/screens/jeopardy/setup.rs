use egui::{Align2, Key};
use crate::screens::kit::{self, Btn, Canvas, TagStyle, Texture};
use crate::theme::{self, INK, MUTED};

#[derive(Clone, PartialEq)]
pub enum JeopardyMode {
    Solo,
    Teams,
}

#[derive(Clone)]
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
        kit::backdrop(ui.ctx(), Some(Texture::Grid));
        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        let c = Canvas::new(ui.ctx());
        let p = ui.painter().clone();
        kit::tag(&p, &c, c.p(80.0, 260.0), "SOLO", TagStyle::Gold, theme::TYPE_TAG);
        p.text(c.p(76.0, 300.0), Align2::LEFT_TOP, "JEOPARDY!", c.cond(800, theme::TYPE_MODE_HERO), INK);
        kit::wrapped(&p, c.p(80.0, 420.0), "Six categories, thirty clues, one Daily Double. Wrong answers cost you.",
            c.barlow(500, theme::TYPE_BODY), MUTED, c.v(520.0), c.v(27.0));
        let mut action = None;
        if kit::button(ui, &c, c.r(80.0, 490.0, 180.0, 58.0), "START GAME", Btn::Primary, "setup_start").clicked()
            || ui.input(|i| i.key_pressed(Key::Enter))
        {
            action = Some(SetupAction::Start(JeopardyConfig {
                mode: JeopardyMode::Solo,
                team_count: 1,
                team_names: vec!["You".to_string()],
            }));
        }
        if kit::button(ui, &c, c.r(272.0, 490.0, 110.0, 58.0), "BACK", Btn::Ghost, "setup_back").clicked()
            || ui.input(|i| i.key_pressed(Key::Escape))
        {
            action = Some(SetupAction::Back);
        }
        action
    }
}
