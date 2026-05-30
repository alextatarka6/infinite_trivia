use egui::{RichText, Stroke, Vec2};
use crate::theme::{self, BLUE_MID, GOLD, INK_DIM, LINE, WHITE};

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
        }
    }
}

impl JeopardySetupScreen {
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<SetupAction> {
        let mut action = None;
        let avail = ui.available_size();

        let content_h = if self.mode == JeopardyMode::Teams {
            420.0_f32
        } else {
            300.0_f32
        };
        let top = ((avail.y - content_h) / 2.0).max(20.0);
        ui.add_space(top);

        ui.vertical_centered(|ui| {
            ui.set_max_width(480.0);

            ui.label(
                RichText::new("JEOPARDY!")
                    .font(egui::FontId::proportional(40.0))
                    .color(GOLD)
                    .strong(),
            );
            ui.label(
                RichText::new("GAME SETUP")
                    .font(theme::mono_font(13.0))
                    .color(INK_DIM),
            );

            ui.add_space(32.0);

            ui.label(
                RichText::new("MODE")
                    .font(theme::mono_font(10.0))
                    .color(INK_DIM),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let btn_w = 140.0_f32;
                let btn_h = 42.0_f32;
                let gap = 10.0_f32;
                let content_w = avail.x.min(480.0);
                ui.add_space((content_w - 2.0 * btn_w - gap) / 2.0);

                let solo_sel = self.mode == JeopardyMode::Solo;
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("SOLO")
                                .font(theme::mono_font(13.0))
                                .color(if solo_sel {
                                    egui::Color32::from_rgb(26, 19, 4)
                                } else {
                                    WHITE
                                }),
                        )
                        .fill(if solo_sel { GOLD } else { egui::Color32::TRANSPARENT })
                        .stroke(Stroke::new(1.5, if solo_sel { GOLD } else { LINE }))
                        .min_size(Vec2::new(btn_w, btn_h)),
                    )
                    .clicked()
                {
                    self.mode = JeopardyMode::Solo;
                }

                ui.add_space(gap);

                let teams_sel = self.mode == JeopardyMode::Teams;
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("TEAMS")
                                .font(theme::mono_font(13.0))
                                .color(if teams_sel {
                                    egui::Color32::from_rgb(26, 19, 4)
                                } else {
                                    WHITE
                                }),
                        )
                        .fill(if teams_sel { GOLD } else { egui::Color32::TRANSPARENT })
                        .stroke(Stroke::new(1.5, if teams_sel { GOLD } else { LINE }))
                        .min_size(Vec2::new(btn_w, btn_h)),
                    )
                    .clicked()
                {
                    self.mode = JeopardyMode::Teams;
                }
            });

            if self.mode == JeopardyMode::Teams {
                ui.add_space(28.0);

                ui.label(
                    RichText::new("NUMBER OF TEAMS")
                        .font(theme::mono_font(10.0))
                        .color(INK_DIM),
                );
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let btn_sz = Vec2::new(36.0, 36.0);
                    let content_w = avail.x.min(480.0);
                    // center the ◀ N ▶ cluster (roughly 36+10+20+10+36 = 112 wide)
                    ui.add_space((content_w - 112.0) / 2.0);

                    let dec = ui.add(
                        egui::Button::new(RichText::new("◀").color(WHITE))
                            .fill(BLUE_MID)
                            .stroke(Stroke::new(1.0, LINE))
                            .min_size(btn_sz),
                    );
                    if dec.clicked() && self.team_count > 2 {
                        self.team_count -= 1;
                    }

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!("{}", self.team_count))
                            .font(egui::FontId::proportional(28.0))
                            .color(WHITE),
                    );
                    ui.add_space(10.0);

                    let inc = ui.add(
                        egui::Button::new(RichText::new("▶").color(WHITE))
                            .fill(BLUE_MID)
                            .stroke(Stroke::new(1.0, LINE))
                            .min_size(btn_sz),
                    );
                    if inc.clicked() && self.team_count < 4 {
                        self.team_count += 1;
                    }
                });

                ui.add_space(20.0);

                ui.label(
                    RichText::new("TEAM NAMES")
                        .font(theme::mono_font(10.0))
                        .color(INK_DIM),
                );
                ui.add_space(8.0);

                let content_w = avail.x.min(480.0);
                for i in 0..self.team_count as usize {
                    ui.horizontal(|ui| {
                        ui.add_space((content_w - 300.0) / 2.0);
                        ui.label(
                            RichText::new(format!("{}.", i + 1))
                                .font(theme::mono_font(11.0))
                                .color(INK_DIM),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.team_names[i])
                                .desired_width(260.0)
                                .font(theme::mono_font(14.0))
                                .text_color(WHITE),
                        );
                    });
                    ui.add_space(6.0);
                }
            }

            ui.add_space(32.0);

            ui.horizontal(|ui| {
                let start_w = 160.0_f32;
                let back_w = 100.0_f32;
                let gap = 12.0_f32;
                let content_w = avail.x.min(480.0);
                ui.add_space((content_w - start_w - back_w - gap) / 2.0);

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Start Game")
                                .color(egui::Color32::from_rgb(26, 19, 4))
                                .font(theme::body_font()),
                        )
                        .fill(GOLD)
                        .min_size(Vec2::new(start_w, 42.0)),
                    )
                    .clicked()
                {
                    let team_names: Vec<String> = (0..self.team_count as usize)
                        .map(|i| {
                            let s = self.team_names[i].trim().to_string();
                            if s.is_empty() {
                                format!("Team {}", i + 1)
                            } else {
                                s
                            }
                        })
                        .collect();
                    action = Some(SetupAction::Start(JeopardyConfig {
                        mode: self.mode.clone(),
                        team_count: self.team_count,
                        team_names,
                    }));
                }

                ui.add_space(gap);

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Back").color(INK_DIM).font(theme::body_font()),
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0, LINE))
                        .min_size(Vec2::new(back_w, 42.0)),
                    )
                    .clicked()
                {
                    action = Some(SetupAction::Back);
                }
            });
        });

        action
    }
}
