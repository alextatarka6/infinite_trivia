use egui::RichText;
use crate::game::state::Screen;
use crate::theme;

pub struct HomeScreen;

impl HomeScreen {
    pub fn show(ui: &mut egui::Ui) -> Option<Screen> {
        let mut next = None;

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(theme::BLUE_BG))
            .show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.label(
                        RichText::new("INFINITE TRIVIA")
                            .font(egui::FontId::proportional(42.0))
                            .color(theme::GOLD)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Select a game mode")
                            .font(theme::subheading_font())
                            .color(theme::WHITE),
                    );
                    ui.add_space(40.0);

                    let btn_size = egui::vec2(280.0, 70.0);

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Jeopardy!")
                                    .font(theme::subheading_font())
                                    .color(theme::GOLD),
                            )
                            .min_size(btn_size)
                            .fill(theme::BLUE_DARK),
                        )
                        .clicked()
                    {
                        next = Some(Screen::Jeopardy);
                    }
                    ui.add_space(12.0);

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("General Trivia")
                                    .font(theme::subheading_font())
                                    .color(theme::GOLD),
                            )
                            .min_size(btn_size)
                            .fill(theme::BLUE_DARK),
                        )
                        .clicked()
                    {
                        next = Some(Screen::Trivia);
                    }
                    ui.add_space(12.0);

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Quizbowl")
                                    .font(theme::subheading_font())
                                    .color(theme::GOLD),
                            )
                            .min_size(btn_size)
                            .fill(theme::BLUE_DARK),
                        )
                        .clicked()
                    {
                        next = Some(Screen::Quizbowl);
                    }
                });
            });

        next
    }
}
