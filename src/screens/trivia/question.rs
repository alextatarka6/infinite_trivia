use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use crate::api::opentdb::TriviaQuestion;
use crate::game::scoring::trivia_score;
use crate::theme::{
    self, ACCENT_SOFT, BLUE_MID, CORRECT_BG, GOLD, GOLD_HOVER, GREEN_CORRECT, INK_DIM,
    LINE, RED_WRONG, WHITE, WRONG_BG,
};

pub enum TriviaAction {
    Next { score_delta: i32 },
    Quit,
}

pub struct TriviaScreen {
    pub questions: Vec<TriviaQuestion>,
    pub current_idx: usize,
    pub score: i32,
    pub streak: u32,
    pub best_streak: u32,
    pub correct_count: u32,
    pub selected: Option<usize>,
    pub timer: f32,
    pub timer_max: f32,
    pub loading: bool,
    pub error: Option<String>,
}

impl TriviaScreen {
    pub fn new(questions: Vec<TriviaQuestion>) -> Self {
        Self {
            questions,
            current_idx: 0,
            score: 0,
            streak: 0,
            best_streak: 0,
            correct_count: 0,
            selected: None,
            timer: 20.0,
            timer_max: 20.0,
            loading: false,
            error: None,
        }
    }

    pub fn loading() -> Self {
        let mut s = Self::new(vec![]);
        s.loading = true;
        s
    }

    pub fn with_error(msg: String) -> Self {
        let mut s = Self::new(vec![]);
        s.error = Some(msg);
        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, dt: f32) -> Option<TriviaAction> {
        let mut action = None;

        // -- Loading ----------------------------------------------------------
        if self.loading {
            let avail_h = ui.available_height();
            ui.add_space((avail_h - 80.0) / 2.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Pulling fresh questions...")
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
                    RichText::new("Could not fetch questions")
                        .font(theme::subheading_font())
                        .color(RED_WRONG),
                );
                ui.label(RichText::new(&err).color(WHITE));
                ui.add_space(16.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Back to menu")
                                .color(Color32::from_rgb(26, 19, 4)),
                        )
                        .fill(GOLD)
                        .min_size(Vec2::new(140.0, 36.0)),
                    )
                    .clicked()
                {
                    action = Some(TriviaAction::Quit);
                }
            });
            return action;
        }

        // -- Round complete ---------------------------------------------------
        if self.current_idx >= self.questions.len() && !self.questions.is_empty() {
            let avail = ui.available_size();
            ui.add_space((avail.y - 320.0).max(16.0) / 2.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("ROUND COMPLETE")
                        .font(theme::mono_font(13.0))
                        .color(GOLD),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("{}", self.score))
                        .font(FontId::proportional(68.0))
                        .color(GOLD)
                        .strong(),
                );
                ui.label(
                    RichText::new("FINAL SCORE")
                        .font(theme::mono_font(11.0))
                        .color(INK_DIM),
                );
                ui.add_space(28.0);
                ui.horizontal(|ui| {
                    for (val, label) in [
                        (self.correct_count.to_string(), "correct"),
                        (
                            (self.questions.len() as u32 - self.correct_count).to_string(),
                            "misses",
                        ),
                        (format!("x{}", self.best_streak), "best streak"),
                    ] {
                        ui.vertical_centered(|ui| {
                            ui.set_min_width(90.0);
                            ui.label(
                                RichText::new(&val)
                                    .font(FontId::proportional(30.0))
                                    .color(WHITE),
                            );
                            ui.label(
                                RichText::new(label)
                                    .font(theme::mono_font(10.0))
                                    .color(INK_DIM),
                            );
                        });
                    }
                });
                ui.add_space(30.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Back to menu")
                                .color(Color32::from_rgb(26, 19, 4))
                                .font(theme::body_font()),
                        )
                        .fill(GOLD)
                        .min_size(Vec2::new(150.0, 38.0)),
                    )
                    .clicked()
                {
                    action = Some(TriviaAction::Quit);
                }
            });
            return action;
        }

        if self.questions.is_empty() {
            return action;
        }

        let q = &self.questions[self.current_idx];
        let answers = q.shuffled_answers();

        // Tick timer
        if self.selected.is_none() {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.timer = 0.0;
                self.streak = 0;
                self.selected = Some(usize::MAX);
            }
        }

        let avail_w = ui.available_width();
        let inner_w = (avail_w - 88.0).min(580.0);
        let left_pad = (avail_w - inner_w) / 2.0;

        ui.add_space(16.0);

        // -- Top bar ----------------------------------------------------------
        let bar_h = 30.0_f32;
        let (bar_rect, _) =
            ui.allocate_exact_size(Vec2::new(avail_w, bar_h), Sense::hover());

        ui.painter().text(
            Pos2::new(left_pad, bar_rect.center().y),
            Align2::LEFT_CENTER,
            format!("Q {}/{} | {}", self.current_idx + 1, self.questions.len(), q.category),
            theme::mono_font(12.0),
            INK_DIM,
        );

        // Streak chip
        if self.streak > 1 {
            let streak_w = 90.0_f32;
            let streak_rect = Rect::from_center_size(
                Pos2::new(left_pad + inner_w - streak_w - 96.0, bar_rect.center().y),
                Vec2::new(streak_w, 26.0),
            );
            ui.painter().rect_filled(streak_rect, Rounding::same(999.0), ACCENT_SOFT);
            ui.painter().rect_stroke(
                streak_rect,
                Rounding::same(999.0),
                Stroke::new(1.0, GOLD),
            );
            ui.painter().text(
                streak_rect.center(),
                Align2::CENTER_CENTER,
                format!("STREAK x{}", self.streak),
                theme::mono_font(10.0),
                GOLD,
            );
        }

        // Score chip
        let chip_w = 88.0_f32;
        let score_chip_rect = Rect::from_center_size(
            Pos2::new(left_pad + inner_w - chip_w / 2.0, bar_rect.center().y),
            Vec2::new(chip_w, 26.0),
        );
        ui.painter().rect_filled(score_chip_rect, Rounding::same(999.0), BLUE_MID);
        ui.painter().rect_stroke(
            score_chip_rect,
            Rounding::same(999.0),
            Stroke::new(1.0, LINE),
        );
        ui.painter().text(
            Pos2::new(score_chip_rect.center().x - 4.0, score_chip_rect.center().y),
            Align2::RIGHT_CENTER,
            "SCORE",
            theme::mono_font(10.0),
            INK_DIM,
        );
        ui.painter().text(
            Pos2::new(score_chip_rect.center().x, score_chip_rect.center().y),
            Align2::LEFT_CENTER,
            format!("  {}", self.score),
            FontId::proportional(13.0),
            WHITE,
        );

        ui.add_space(10.0);

        // -- Timer bar --------------------------------------------------------
        let ratio = (self.timer / self.timer_max).clamp(0.0, 1.0);
        let bar_color = if ratio > 0.5 {
            GREEN_CORRECT
        } else if ratio > 0.25 {
            GOLD
        } else {
            RED_WRONG
        };
        let (timer_rect, _) =
            ui.allocate_exact_size(Vec2::new(avail_w, 8.0), Sense::hover());
        ui.painter().rect_filled(timer_rect, Rounding::same(4.0), BLUE_MID);
        let mut filled = timer_rect;
        filled.set_right(timer_rect.left() + timer_rect.width() * ratio);
        ui.painter().rect_filled(filled, Rounding::same(4.0), bar_color);

        ui.add_space(18.0);

        // -- Question ---------------------------------------------------------
        ui.vertical_centered(|ui| {
            ui.set_max_width(inner_w);
            ui.label(
                RichText::new(&q.question)
                    .font(FontId::proportional(24.0))
                    .color(WHITE),
            );
        });

        ui.add_space(18.0);

        // -- Answer options ---------------------------------------------------
        let opt_h = 52.0_f32;
        let opt_gap = 9.0_f32;
        let mut clicked_idx: Option<usize> = None;

        for (i, (ans_text, is_correct)) in answers.iter().enumerate() {
            // Allocate full width to advance cursor; draw only in the centered inner rect
            let (alloc_rect, resp) = ui.allocate_exact_size(
                Vec2::new(avail_w, opt_h),
                if self.selected.is_none() { Sense::click() } else { Sense::hover() },
            );
            let draw_rect = Rect::from_min_size(
                Pos2::new(alloc_rect.min.x + left_pad, alloc_rect.min.y),
                Vec2::new(inner_w, opt_h),
            );

            let (bg, border, text_color, badge_bg, badge_text) = if let Some(sel) = self.selected {
                if *is_correct {
                    (CORRECT_BG, GREEN_CORRECT, GREEN_CORRECT, GREEN_CORRECT, Color32::from_rgb(12, 26, 16))
                } else if sel == i {
                    (WRONG_BG, RED_WRONG, RED_WRONG, RED_WRONG, Color32::from_rgb(42, 12, 17))
                } else {
                    (Color32::TRANSPARENT, LINE, Color32::from_rgba_premultiplied(75, 79, 100, 140), BLUE_MID, INK_DIM)
                }
            } else if resp.hovered() {
                (BLUE_MID, GOLD, WHITE, BLUE_MID, INK_DIM)
            } else {
                (BLUE_MID, LINE, WHITE, BLUE_MID, INK_DIM)
            };

            ui.painter().rect_filled(draw_rect, Rounding::same(10.0), bg);
            ui.painter().rect_stroke(draw_rect, Rounding::same(10.0), Stroke::new(1.0, border));

            // Key badge
            let badge_size = 26.0_f32;
            let badge_rect = Rect::from_min_size(
                Pos2::new(draw_rect.min.x + 14.0, draw_rect.center().y - badge_size / 2.0),
                Vec2::splat(badge_size),
            );
            ui.painter().rect_filled(badge_rect, Rounding::same(6.0), badge_bg);
            if self.selected.is_none() {
                ui.painter().rect_stroke(
                    badge_rect,
                    Rounding::same(6.0),
                    Stroke::new(1.0, LINE),
                );
            }
            ui.painter().text(
                badge_rect.center(),
                Align2::CENTER_CENTER,
                &char::from(b'A' + i as u8).to_string(),
                theme::mono_font(12.0),
                badge_text,
            );

            // Answer text
            ui.painter().text(
                Pos2::new(badge_rect.right() + 12.0, draw_rect.center().y),
                Align2::LEFT_CENTER,
                ans_text.as_str(),
                theme::body_font(),
                text_color,
            );

            // Checkmark
            if self.selected.is_some() && *is_correct {
                ui.painter().text(
                    Pos2::new(draw_rect.right() - 16.0, draw_rect.center().y),
                    Align2::RIGHT_CENTER,
                    "OK",
                    FontId::proportional(16.0),
                    GREEN_CORRECT,
                );
            }

            if resp.clicked() && self.selected.is_none() {
                clicked_idx = Some(i);
            }

            if i + 1 < answers.len() {
                ui.add_space(opt_gap);
            }
        }

        // Apply answer selection
        if let Some(i) = clicked_idx {
            let is_correct = answers[i].1;
            self.selected = Some(i);
            let delta = trivia_score(is_correct, self.streak);
            self.score += delta;
            if is_correct {
                self.streak += 1;
                self.correct_count += 1;
                if self.streak > self.best_streak {
                    self.best_streak = self.streak;
                }
            } else {
                self.streak = 0;
            }
        }

        ui.add_space(16.0);

        // -- Footer: gain label + next button ---------------------------------
        let (foot_rect, _) =
            ui.allocate_exact_size(Vec2::new(avail_w, 46.0), Sense::hover());

        if let Some(sel) = self.selected {
            let timed = sel == usize::MAX;

            let (gain_str, gain_color) = if timed {
                ("time!".to_string(), RED_WRONG)
            } else if answers[sel].1 {
                let g = trivia_score(true, self.streak.saturating_sub(1));
                (format!("+{}", g), GREEN_CORRECT)
            } else {
                ("+0".to_string(), RED_WRONG)
            };

            ui.painter().text(
                Pos2::new(left_pad, foot_rect.center().y),
                Align2::LEFT_CENTER,
                &gain_str,
                FontId::proportional(18.0),
                gain_color,
            );

            let btn_w = 110.0_f32;
            let btn_rect = Rect::from_center_size(
                Pos2::new(left_pad + inner_w - btn_w / 2.0, foot_rect.center().y),
                Vec2::new(btn_w, 38.0),
            );
            let btn_resp = ui.allocate_rect(btn_rect, Sense::click());
            let btn_fill = if btn_resp.hovered() { GOLD_HOVER } else { GOLD };
            ui.painter().rect_filled(btn_rect, Rounding::same(999.0), btn_fill);
            let btn_label = if self.current_idx + 1 < self.questions.len() {
                "Next >"
            } else {
                "Finish >"
            };
            ui.painter().text(
                btn_rect.center(),
                Align2::CENTER_CENTER,
                btn_label,
                theme::body_font(),
                Color32::from_rgb(26, 19, 4),
            );
            if btn_resp.clicked() {
                self.current_idx += 1;
                self.selected = None;
                self.timer = self.timer_max;
                action = Some(TriviaAction::Next { score_delta: 0 });
            }
        }

        // -- Quit button ------------------------------------------------------
        let quit_rect =
            Rect::from_min_size(Pos2::new(avail_w - 90.0, 14.0), Vec2::new(76.0, 30.0));
        let quit_resp = ui.allocate_rect(quit_rect, Sense::click());
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
            action = Some(TriviaAction::Quit);
        }

        action
    }
}
