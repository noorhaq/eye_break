use crate::config::Config;
use crate::exercises::{self, Exercise};
use crate::monitors::MonitorRect;
use crate::raise;
use crate::state::{self, State};
use eframe::egui;
use std::time::{Duration, Instant};

/// How often to poll the real (root-window) mouse position for the
/// click-through hit-test, in milliseconds. Independent of the ~33ms paint
/// loop below — each poll shells out to `xdotool`, so this stays coarser
/// than every frame. 50ms is still well under normal hover-before-click
/// reaction time, so the buttons don't feel laggy to reach.
const MOUSE_POLL_MS: u64 = 50;

pub fn run_overlay(
    rect: MonitorRect,
    display_secs: f32,
    exercise_index: usize,
    original_focus: Option<String>,
    is_long_break: bool,
) -> eframe::Result<()> {
    raise::keep_on_top_in_background(original_focus);

    let viewport = egui::ViewportBuilder::default()
        .with_position(egui::pos2(rect.x as f32, rect.y as f32))
        .with_inner_size(egui::vec2(rect.w as f32, rect.h as f32))
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_taskbar(false)
        .with_resizable(false)
        // Click-through by default — see OverlayApp's mouse-passthrough
        // handling below for why, and how the buttons still work despite
        // this. Without it, the overlay (a normal top-level window covering
        // the *entire* monitor) would swallow every click anywhere on
        // screen for the whole break, not just clicks on its own buttons —
        // exactly the "it takes over the mouse" complaint.
        .with_mouse_passthrough(true);

    let options = eframe::NativeOptions {
        viewport,
        centered: false,
        ..Default::default()
    };

    eframe::run_native(
        "eye-break-overlay",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(OverlayApp::new(rect, display_secs, exercise_index, is_long_break)))
        }),
    )
}

struct OverlayApp {
    rect: MonitorRect,
    start: Instant,
    display_secs: f32,
    exercise: &'static Exercise,
    reminder_text: String,
    is_long_break: bool,
    // Loaded once at startup, same as `reminder_text` — a break already in
    // progress keeps whatever mode it started in even if Settings changes
    // strict mode mid-break, which is simpler to reason about than a break
    // that suddenly grows or loses its buttons partway through.
    strict_mode: bool,
    my_dismiss_token: u64,
    last_state_poll: Instant,
    last_mouse_poll: Instant,
    mouse_pos: Option<(f32, f32)>,
    passthrough: bool,
}

impl OverlayApp {
    fn new(rect: MonitorRect, display_secs: f32, exercise_index: usize, is_long_break: bool) -> Self {
        let my_dismiss_token = State::load().dismiss_token;
        let cfg = Config::load();
        Self {
            rect,
            start: Instant::now(),
            display_secs,
            exercise: exercises::get(exercise_index),
            reminder_text: cfg.reminder_text,
            is_long_break,
            strict_mode: cfg.strict_mode,
            my_dismiss_token,
            last_state_poll: Instant::now(),
            last_mouse_poll: Instant::now() - Duration::from_millis(MOUSE_POLL_MS),
            mouse_pos: None,
            // Matches the ViewportBuilder default above; kept in sync with
            // the actual window state by set_passthrough below rather than
            // re-sent unconditionally every frame.
            passthrough: true,
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

    /// "OK, I'm done" — dismiss right now without waiting out the timer, but
    /// (unlike Skip) without pushing the next break out by the snooze
    /// length. `trigger_break` already stamped `last_break_epoch` when this
    /// overlay was spawned, so the next break stays on its normal schedule;
    /// this just closes the window(s) early. Bumping `dismiss_token` closes
    /// sibling overlays on other monitors too, same as Skip does.
    fn acknowledge(&self) {
        let mut st = State::load();
        st.dismiss_token += 1;
        st.save();
    }

    /// Re-queries the real mouse position (throttled) via `xdotool`, since
    /// egui's own pointer tracking goes blind the instant mouse
    /// pass-through is engaged — see `raise::global_mouse_pos`.
    fn poll_mouse(&mut self) {
        if self.last_mouse_poll.elapsed() < Duration::from_millis(MOUSE_POLL_MS) {
            return;
        }
        self.last_mouse_poll = Instant::now();
        self.mouse_pos = raise::global_mouse_pos();
    }

    /// Enables mouse pass-through everywhere on this window except while
    /// the cursor sits over one of `hit_rects` (button bounds, in this
    /// window's local/logical coordinates) — so the break overlay never
    /// blocks clicks into whatever the user was doing underneath, while its
    /// own buttons stay clickable. Only actually sends the viewport command
    /// on a change, not every frame.
    fn update_passthrough(&mut self, ctx: &egui::Context, hit_rects: &[egui::Rect]) {
        let ppp = ctx.pixels_per_point();
        // Where the window manager *actually* placed this window, not where
        // we asked for it to go — Mutter, at least, nudges an undecorated
        // window requested at (0, 0) down below the top panel (observed:
        // requested (0, 0), landed at (0, 32)) despite always-on-top and no
        // decorations. Using the requested `rect.x/y` here instead of this
        // reads the mouse position ~32px off from where egui itself thinks
        // it is, and the button hit-test silently never matches.
        let origin = ctx
            .input(|i| i.viewport().inner_rect)
            .map(|r| r.min)
            .unwrap_or(egui::pos2(self.rect.x as f32, self.rect.y as f32));
        let hovering = self.mouse_pos.is_some_and(|(mx, my)| {
            // xdotool reports physical root-window pixels; hit_rects are in
            // this window's logical points, offset by the window's own
            // actual on-screen position.
            let local = egui::pos2(mx / ppp - origin.x, my / ppp - origin.y);
            hit_rects.iter().any(|r| r.contains(local))
        });

        let want_passthrough = !hovering;
        if want_passthrough != self.passthrough {
            self.passthrough = want_passthrough;
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(want_passthrough));
        }
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

        self.poll_mouse();

        // Fade in/out slightly at the edges for a softer feel.
        let fade = 0.3_f32.min(self.display_secs / 4.0);
        let alpha = if elapsed < fade {
            elapsed / fade
        } else if remaining < fade {
            remaining / fade
        } else {
            1.0
        };

        let mut hit_rects: Vec<egui::Rect> = Vec::with_capacity(2);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgba_unmultiplied(
                0,
                0,
                0,
                // While click-through, don't dim/cover the screen at all —
                // only the tiny hovered-button hit box is ever "solid" from
                // the window manager's perspective, but the dim fill is
                // still painted every frame for the visual effect
                // regardless of passthrough state (passthrough only
                // affects input routing, not rendering).
                (180.0 * alpha) as u8,
            )))
            .show(ctx, |ui| {
                let screen = ui.max_rect();
                let center = screen.center();
                let line_count = self.exercise.lines.len() as f32;
                let body_top = center.y - 10.0 - (line_count - 1.0) * 14.0;

                if self.is_long_break {
                    ui.painter().text(
                        egui::pos2(center.x, center.y - 168.0),
                        egui::Align2::CENTER_CENTER,
                        "LONG BREAK",
                        egui::FontId::proportional(13.0),
                        egui::Color32::from_rgba_unmultiplied(255, 210, 120, (200.0 * alpha) as u8),
                    );
                }

                ui.painter().text(
                    egui::pos2(center.x, center.y - 140.0),
                    egui::Align2::CENTER_CENTER,
                    &self.reminder_text,
                    egui::FontId::proportional(28.0),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, (230.0 * alpha) as u8),
                );

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

                if self.strict_mode {
                    // No way out early — this break runs for the full
                    // `display_secs`/`long_break_display_secs` no matter
                    // what. Say so, in place of the buttons, rather than
                    // just silently not offering them.
                    ui.painter().text(
                        egui::pos2(center.x, screen.bottom() - 60.0),
                        egui::Align2::CENTER_CENTER,
                        "Strict mode — this break can't be skipped",
                        egui::FontId::proportional(14.0),
                        egui::Color32::from_rgba_unmultiplied(180, 180, 180, (200.0 * alpha) as u8),
                    );
                } else {
                    // Two buttons side by side, centered near the bottom:
                    // "OK, I'm done" dismisses right now with no effect on
                    // the next break's schedule; "Skip" also dismisses now
                    // but pushes the next break out by the snooze length.
                    let ok_size = egui::vec2(180.0, 40.0);
                    let skip_size = egui::vec2(230.0, 40.0);
                    let gap = 12.0;
                    let total_w = ok_size.x + gap + skip_size.x;
                    let row_y = screen.bottom() - 70.0;
                    let ok_rect = egui::Rect::from_center_size(
                        egui::pos2(center.x - total_w / 2.0 + ok_size.x / 2.0, row_y),
                        ok_size,
                    );
                    let skip_rect = egui::Rect::from_center_size(
                        egui::pos2(center.x + total_w / 2.0 - skip_size.x / 2.0, row_y),
                        skip_size,
                    );
                    // A little slack around the visible button bounds so the
                    // passthrough toggle (driven by a throttled, external
                    // mouse poll rather than egui's own now-blind pointer
                    // tracking) engages a beat before the cursor is exactly
                    // on the edge.
                    hit_rects.push(ok_rect.expand(6.0));
                    hit_rects.push(skip_rect.expand(6.0));

                    let text_alpha = (255.0 * alpha) as u8;
                    let make_button = |label: String, fill: u8| {
                        egui::Button::new(
                            egui::RichText::new(label)
                                .size(16.0)
                                .color(egui::Color32::from_rgba_unmultiplied(
                                    255, 255, 255, text_alpha,
                                )),
                        )
                        .fill(egui::Color32::from_rgba_unmultiplied(
                            255,
                            255,
                            255,
                            (fill as f32 * alpha) as u8,
                        ))
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, (120.0 * alpha) as u8),
                        ))
                        .rounding(8.0)
                    };

                    // "OK" is the primary action — filled brighter so it
                    // reads as the default choice for someone who just wants
                    // to move on.
                    if ui
                        .put(ok_rect, make_button("OK, I'm done".to_string(), 70))
                        .clicked()
                    {
                        self.acknowledge();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        std::process::exit(0);
                    }

                    let snooze_min = Config::load().snooze_secs / 60;
                    if ui
                        .put(
                            skip_rect,
                            make_button(format!("Skip — remind me in {snooze_min} min"), 30),
                        )
                        .clicked()
                    {
                        self.skip();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        std::process::exit(0);
                    }
                }
            });

        self.update_passthrough(ctx, &hit_rects);

        ctx.request_repaint_after(Duration::from_millis(33));
    }
}
