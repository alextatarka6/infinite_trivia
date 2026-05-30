use egui::{Align2, Color32, FontId, Id, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use crate::api::qbreader::{check_answer, Tossup};
use crate::game::scoring::quizbowl_score;
use crate::theme::{
    self, BLUE_MID, GOLD, GOLD_HOVER, GREEN_CORRECT, INK_DIM, LINE, RED_WRONG, WHITE,
};

const CHARS_PER_SECOND: f32 = 20.0;

pub enum TossupAction {
    Next,
    Quit,
}

pub enum TossupState {
    Reading,
    FinishedReading { elapsed: f32 },
    BuzzedIn { answer_input: String },
    Result { correct: bool, score_delta: i32 },
    TimedOut,
}

pub struct TossupScreen {
    pub tossup: Tossup,
    pub score: i32,
    pub questions_done: u32,
    pub chars_revealed: usize,
    pub char_timer: f32,
    pub state: TossupState,
    pub loading: bool,
    pub error: Option<String>,
}

impl TossupScreen {
    pub fn new(tossup: Tossup, score: i32, questions_done: u32) -> Self {
        Self {
            tossup,
            score,
            questions_done,
            chars_revealed: 0,
            char_timer: 0.0,
            state: TossupState::Reading,
            loading: false,
            error: None,
        }
    }

    pub fn loading(score: i32, questions_done: u32) -> Self {
        let dummy = Tossup {
            question: String::new(),
            answer: String::new(),
            accepted_answers: vec![],
            category: String::new(),
            difficulty: String::new(),
        };
        let mut s = Self::new(dummy, score, questions_done);
        s.loading = true;
        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, dt: f32) -> Option<TossupAction> {
        let mut action = None;

        // -- Loading ----------------------------------------------------------
        if self.loading {
            let avail_h = ui.available_height();
            ui.add_space((avail_h - 80.0) / 2.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Finding a tossup...")
                        .font(FontId::proportional(20.0))
                        .color(INK_DIM)
                        .italics(),
                );
            });
            return None;
        }

        // -- Error ------------------------------------------------------------
        if let Some(err) = self.error.clone() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    RichText::new("Could not fetch question")
                        .font(theme::subheading_font())
                        .color(RED_WRONG),
                );
                ui.label(RichText::new(&err).color(WHITE));
                ui.add_space(16.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Back to menu")
                                .color(Color32::from_rgb(26, 19, 4))
                                .font(theme::body_font()),
                        )
                        .fill(GOLD)
                        .min_size(Vec2::new(140.0, 36.0)),
                    )
                    .clicked()
                {
                    action = Some(TossupAction::Quit);
                }
            });
            return action;
        }

        let total_chars = self.tossup.question.chars().count();

        // -- Advance reveal ---------------------------------------------------
        if matches!(self.state, TossupState::Reading) {
            self.char_timer += dt;
            let target = (self.char_timer * CHARS_PER_SECOND) as usize;
            self.chars_revealed = target.min(total_chars);
            if self.chars_revealed >= total_chars {
                self.state = TossupState::FinishedReading { elapsed: 0.0 };
            }
        }
        if let TossupState::FinishedReading { ref mut elapsed } = self.state {
            *elapsed += dt;
            if *elapsed >= 5.0 {
                self.state = TossupState::TimedOut;
            }
        }

        let revealed_text: String =
            self.tossup.question.chars().take(self.chars_revealed).collect();
        let remaining_text: String =
            self.tossup.question.chars().skip(self.chars_revealed).collect();

        let avail_w = ui.available_width();
        let content_pad = 48.0_f32;
        let inner_w = (avail_w - content_pad * 2.0).min(680.0);
        let left_pad = (avail_w - inner_w) / 2.0;

        ui.add_space(16.0);

        // -- Top bar ----------------------------------------------------------
        let bar_h = 30.0_f32;
        let (bar_rect, _) =
            ui.allocate_exact_size(Vec2::new(avail_w, bar_h), Sense::hover());

        ui.painter().text(
            Pos2::new(left_pad, bar_rect.center().y),
            Align2::LEFT_CENTER,
            format!(
                "QUIZBOWL  |  {}  |  {}",
                self.tossup.category.to_uppercase(),
                self.tossup.difficulty.to_uppercase()
            ),
            theme::mono_font(11.0),
            GOLD,
        );

        let chip_w = 104.0_f32;
        let chip_rect = Rect::from_center_size(
            Pos2::new(left_pad + inner_w - chip_w / 2.0, bar_rect.center().y),
            Vec2::new(chip_w, 26.0),
        );
        ui.painter().rect_filled(chip_rect, Rounding::same(999.0), BLUE_MID);
        ui.painter().rect_stroke(chip_rect, Rounding::same(999.0), Stroke::new(1.0, LINE));
        ui.painter().text(
            Pos2::new(chip_rect.center().x - 4.0, chip_rect.center().y),
            Align2::RIGHT_CENTER,
            format!("Q{}", self.questions_done + 1),
            theme::mono_font(10.0),
            INK_DIM,
        );
        ui.painter().text(
            Pos2::new(chip_rect.center().x, chip_rect.center().y),
            Align2::LEFT_CENTER,
            format!("  {} pts", self.score),
            FontId::proportional(13.0),
            WHITE,
        );

        ui.add_space(18.0);

        // -- Question text ----------------------------------------------------
        let is_result = matches!(self.state, TossupState::Result { .. });
        ui.vertical_centered(|ui| {
            ui.set_max_width(inner_w);
            let mut job = egui::text::LayoutJob::default();
            job.append(
                &revealed_text,
                0.0,
                egui::text::TextFormat {
                    font_id: FontId::proportional(24.0),
                    color: WHITE,
                    ..Default::default()
                },
            );
            if is_result && !remaining_text.is_empty() {
                job.append(
                    &remaining_text,
                    0.0,
                    egui::text::TextFormat {
                        font_id: FontId::proportional(24.0),
                        color: Color32::from_rgba_premultiplied(94, 100, 126, 140),
                        ..Default::default()
                    },
                );
            }
            ui.label(job);
        });

        ui.add_space(20.0);

        // -- Collect UI events (state-specific rendering) ---------------------
        let mut buzz_clicked = false;
        let mut answer_submitted: Option<String> = None;
        let mut next_clicked = false;

        // Reading / FinishedReading: buzz button
        if matches!(self.state, TossupState::Reading | TossupState::FinishedReading { .. }) {
            let buzz_w = 220.0_f32;
            let buzz_h = 54.0_f32;
            let (buzz_area, _) =
                ui.allocate_exact_size(Vec2::new(avail_w, buzz_h), Sense::hover());
            let buzz_rect = Rect::from_center_size(
                Pos2::new(avail_w / 2.0, buzz_area.center().y),
                Vec2::new(buzz_w, buzz_h),
            );
            let buzz_resp =
                ui.interact(buzz_rect, Id::new("qb_buzz"), Sense::click());
            let fill = if buzz_resp.hovered() { GOLD_HOVER } else { GOLD };
            ui.painter().rect_filled(buzz_rect, Rounding::same(14.0), fill);
            ui.painter().text(
                Pos2::new(buzz_rect.center().x - 24.0, buzz_rect.center().y),
                Align2::RIGHT_CENTER,
                "BUZZ IN",
                FontId::proportional(18.0),
                Color32::from_rgb(26, 19, 4),
            );
            let key_rect = Rect::from_center_size(
                Pos2::new(buzz_rect.center().x + 30.0, buzz_rect.center().y),
                Vec2::new(56.0, 22.0),
            );
            ui.painter().rect_filled(
                key_rect,
                Rounding::same(5.0),
                Color32::from_rgba_premultiplied(0, 0, 0, 46),
            );
            ui.painter().text(
                key_rect.center(),
                Align2::CENTER_CENTER,
                "SPACE",
                theme::mono_font(9.0),
                Color32::from_rgb(26, 19, 4),
            );
            if buzz_resp.clicked() || ui.input(|i| i.key_pressed(egui::Key::Space)) {
                buzz_clicked = true;
            }
        }

        // BuzzedIn: text input
        if let TossupState::BuzzedIn { ref mut answer_input } = self.state {
            let mut to_submit = false;
            ui.horizontal(|ui| {
                let h_margin = (avail_w - 480.0).max(0.0) / 2.0;
                ui.add_space(h_margin);
                ui.label(
                    RichText::new("Answer")
                        .font(theme::mono_font(12.0))
                        .color(GOLD),
                );
                let resp = ui.add(
                    egui::TextEdit::singleline(answer_input)
                        .desired_width(320.0)
                        .font(theme::mono_font(16.0))
                        .text_color(WHITE),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    to_submit = true;
                }
            });
            ui.add_space(12.0);
            ui.vertical_centered(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Submit")
                                .color(Color32::from_rgb(26, 19, 4))
                                .font(theme::body_font()),
                        )
                        .fill(GOLD)
                        .min_size(Vec2::new(100.0, 38.0)),
                    )
                    .clicked()
                {
                    to_submit = true;
                }
            });
            if to_submit && !answer_input.trim().is_empty() {
                answer_submitted = Some(answer_input.clone());
            }
        }

        // TimedOut
        if matches!(self.state, TossupState::TimedOut) {
            ui.vertical_centered(|ui| {
                ui.set_max_width(500.0);
                ui.label(
                    RichText::new("TIME")
                        .font(FontId::proportional(32.0))
                        .color(RED_WRONG)
                        .strong(),
                );
                ui.add_space(8.0);
                egui::Frame::none()
                    .fill(BLUE_MID)
                    .rounding(Rounding::same(12.0))
                    .stroke(Stroke::new(1.0, LINE))
                    .inner_margin(egui::Margin::symmetric(20.0, 12.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("ANSWER")
                                .font(theme::mono_font(10.0))
                                .color(INK_DIM),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&self.tossup.answer)
                                .font(FontId::proportional(20.0))
                                .color(WHITE),
                        );
                    });
                ui.add_space(16.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Next tossup >")
                                .color(Color32::from_rgb(26, 19, 4))
                                .font(theme::body_font()),
                        )
                        .fill(GOLD)
                        .min_size(Vec2::new(150.0, 38.0)),
                    )
                    .clicked()
                {
                    next_clicked = true;
                }
            });
        }

        // Result
        if let TossupState::Result { correct, score_delta } = self.state {
            ui.vertical_centered(|ui| {
                ui.set_max_width(500.0);
                ui.horizontal(|ui| {
                    let label = if correct { "CORRECT" } else { "INCORRECT" };
                    let color = if correct { GREEN_CORRECT } else { RED_WRONG };
                    ui.label(
                        RichText::new(label)
                            .font(FontId::proportional(30.0))
                            .color(color)
                            .strong(),
                    );
                    ui.add_space(12.0);
                    let delta_str = if score_delta >= 0 {
                        format!("+{}", score_delta)
                    } else {
                        score_delta.to_string()
                    };
                    ui.label(
                        RichText::new(delta_str)
                            .font(FontId::proportional(22.0))
                            .color(color),
                    );
                });
                ui.add_space(10.0);
                egui::Frame::none()
                    .fill(BLUE_MID)
                    .rounding(Rounding::same(12.0))
                    .stroke(Stroke::new(1.0, LINE))
                    .inner_margin(egui::Margin::symmetric(20.0, 12.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("ANSWER")
                                .font(theme::mono_font(10.0))
                                .color(INK_DIM),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&self.tossup.answer)
                                .font(FontId::proportional(20.0))
                                .color(WHITE),
                        );
                    });
                ui.add_space(16.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Next tossup >")
                                .color(Color32::from_rgb(26, 19, 4))
                                .font(theme::body_font()),
                        )
                        .fill(GOLD)
                        .min_size(Vec2::new(150.0, 38.0)),
                    )
                    .clicked()
                {
                    next_clicked = true;
                }
            });
        }

        // -- Apply state transitions ------------------------------------------
        if buzz_clicked {
            self.state = TossupState::BuzzedIn { answer_input: String::new() };
        } else if let Some(ans) = answer_submitted {
            let correct = check_answer(&ans, &self.tossup.accepted_answers);
            let delta = quizbowl_score(correct, self.chars_revealed, total_chars);
            self.score += delta;
            self.chars_revealed = total_chars;
            self.state = TossupState::Result { correct, score_delta: delta };
        } else if next_clicked {
            action = Some(TossupAction::Next);
        }

        // -- Quit button ------------------------------------------------------
        let quit_rect =
            Rect::from_min_size(Pos2::new(avail_w - 90.0, 14.0), Vec2::new(76.0, 30.0));
        let quit_resp = ui.interact(quit_rect, Id::new("qb_quit"), Sense::click());
        ui.painter()
            .rect_stroke(quit_rect, Rounding::same(999.0), Stroke::new(1.0, LINE));
        ui.painter().text(
            quit_rect.center(),
            Align2::CENTER_CENTER,
            "Quit",
            theme::body_font(),
            INK_DIM,
        );
        if quit_resp.clicked() {
            action = Some(TossupAction::Quit);
        }

        action
    }
}
