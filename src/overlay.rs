use crate::config::Config;
use crate::exercises::{self, Exercise};
use crate::monitors::MonitorRect;
use crate::state::{self, State};
use eframe::egui;
use std::time::{Duration, Instant};

pub fn run_overlay(
    rect: MonitorRect,
    display_secs: f32,
    exercise_index: usize,
) -> eframe::Result<()> {
    crate::raise::keep_on_top_in_background();

    let viewport = egui::ViewportBuilder::default()
        .with_position(egui::pos2(rect.x as f32, rect.y as f32))
        .with_inner_size(egui::vec2(rect.w as f32, rect.h as f32))
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_taskbar(false)
        .with_resizable(false);

    let options = eframe::NativeOptions {
        viewport,
        centered: false,
        ..Default::default()
    };

    eframe::run_native(
        "eye-break-overlay",
        options,
        Box::new(move |_cc| Ok(Box::new(OverlayApp::new(display_secs, exercise_index)))),
    )
}

struct OverlayApp {
    start: Instant,
    display_secs: f32,
    exercise: &'static Exercise,
    my_dismiss_token: u64,
    last_state_poll: Instant,
}

impl OverlayApp {
    fn new(display_secs: f32, exercise_index: usize) -> Self {
        let my_dismiss_token = State::load().dismiss_token;
        Self {
            start: Instant::now(),
            display_secs,
            exercise: exercises::get(exercise_index),
            my_dismiss_token,
            last_state_poll: Instant::now(),
        }
    }

    /// A sibling overlay (another monitor) may have been skipped; if so,
    /// this window should close too instead of lingering on its own.
    fn was_dismissed_elsewhere(&mut self) -> bool {
        if self.last_state_poll.elapsed() < Duration::from_millis(200) {
            return false;
        }
        self.last_state_poll = Instant::now();
        State::load().dismiss_token != self.my_dismiss_token
    }

    fn skip(&self) {
        let cfg = Config::load();
        let mut st = State::load();
        st.snooze_until_epoch = Some(state::now_epoch() + cfg.snooze_secs);
        st.dismiss_token += 1;
        st.save();
    }
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Fully transparent clear; we paint our own dim overlay so the effect
        // works even without a compositor giving us real window transparency.
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::theme::apply(ctx, Config::load().theme);

        if self.was_dismissed_elsewhere() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            std::process::exit(0);
        }

        let elapsed = self.start.elapsed().as_secs_f32();
        let remaining = (self.display_secs - elapsed).max(0.0);

        if remaining <= 0.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            std::process::exit(0);
        }

        // Fade in/out slightly at the edges for a softer feel.
        let fade = 0.3_f32.min(self.display_secs / 4.0);
        let alpha = if elapsed < fade {
            elapsed / fade
        } else if remaining < fade {
            remaining / fade
        } else {
            1.0
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgba_unmultiplied(
                0,
                0,
                0,
                (180.0 * alpha) as u8,
            )))
            .show(ctx, |ui| {
                let screen = ui.max_rect();
                let center = screen.center();
                let line_count = self.exercise.lines.len() as f32;
                let body_top = center.y - 10.0 - (line_count - 1.0) * 14.0;

                ui.painter().text(
                    egui::pos2(center.x, center.y - 90.0),
                    egui::Align2::CENTER_CENTER,
                    self.exercise.title,
                    egui::FontId::proportional(46.0),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, (255.0 * alpha) as u8),
                );

                for (i, line) in self.exercise.lines.iter().enumerate() {
                    ui.painter().text(
                        egui::pos2(center.x, body_top + i as f32 * 28.0),
                        egui::Align2::CENTER_CENTER,
                        *line,
                        egui::FontId::proportional(22.0),
                        egui::Color32::from_rgba_unmultiplied(
                            230,
                            230,
                            230,
                            (255.0 * alpha) as u8,
                        ),
                    );
                }

                ui.painter().text(
                    egui::pos2(center.x, center.y + 110.0),
                    egui::Align2::CENTER_CENTER,
                    format!("closing in {}s", remaining.ceil() as i32),
                    egui::FontId::proportional(16.0),
                    egui::Color32::from_rgba_unmultiplied(180, 180, 180, (255.0 * alpha) as u8),
                );

                // Skip / snooze button, centered near the bottom.
                let button_size = egui::vec2(260.0, 40.0);
                let button_rect = egui::Rect::from_center_size(
                    egui::pos2(center.x, screen.bottom() - 70.0),
                    button_size,
                );
                let snooze_min = Config::load().snooze_secs / 60;
                let button = egui::Button::new(
                    egui::RichText::new(format!("Skip — remind me in {snooze_min} min"))
                        .size(16.0)
                        .color(egui::Color32::from_rgba_unmultiplied(
                            255,
                            255,
                            255,
                            (255.0 * alpha) as u8,
                        )),
                )
                .fill(egui::Color32::from_rgba_unmultiplied(
                    255,
                    255,
                    255,
                    (30.0 * alpha) as u8,
                ))
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, (120.0 * alpha) as u8),
                ))
                .rounding(8.0);

                if ui.put(button_rect, button).clicked() {
                    self.skip();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    std::process::exit(0);
                }
            });

        ctx.request_repaint_after(Duration::from_millis(33));
    }
}
