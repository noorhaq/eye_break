use crate::config::Config;
use crate::design;
use crate::monitors::primary_monitor;
use crate::pomodoro::{PomodoroConfig, PomodoroState};
use crate::state::{now_epoch, State};
use eframe::egui;
use std::time::{Duration, Instant};

const WIN_W: f32 = 230.0;
const WIN_H: f32 = 62.0;
const MARGIN: f32 = 16.0;

/// A small always-on-top card in the corner of the primary monitor showing
/// a progress ring + "NEXT BREAK IN mm:ss", styled to match the Settings
/// window's Classical design. Runs as its own long-lived process so it
/// doesn't have to share an event loop with the GTK-driven tray icon.
pub fn run_timer() -> eframe::Result<()> {
    crate::raise::keep_on_top_in_background();

    let mon = primary_monitor();
    let x = mon.x as f32 + mon.w as f32 - WIN_W - MARGIN;
    let y = mon.y as f32 + MARGIN;

    let viewport = egui::ViewportBuilder::default()
        .with_position(egui::pos2(x, y))
        .with_inner_size(egui::vec2(WIN_W, WIN_H))
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_taskbar(false)
        .with_resizable(false)
        .with_mouse_passthrough(true);

    let options = eframe::NativeOptions {
        viewport,
        centered: false,
        ..Default::default()
    };

    eframe::run_native(
        "eye-break-timer",
        options,
        Box::new(|cc| {
            design::install_fonts(&cc.egui_ctx);
            Ok(Box::new(TimerApp::new()))
        }),
    )
}

struct TimerApp {
    last_poll: Instant,
    cfg: Config,
    state: State,
    pomodoro_state: PomodoroState,
}

impl TimerApp {
    fn new() -> Self {
        Self {
            last_poll: Instant::now() - Duration::from_secs(10),
            cfg: Config::load(),
            state: State::load(),
            pomodoro_state: PomodoroState::load(),
        }
    }
}

/// Scales a color's RGBA channels uniformly by `factor` (0.0-1.0), giving a
/// correctly-premultiplied faded color — used for the "corner timer
/// opacity" setting, since the card's whole visual weight (not just its
/// background fill) should fade together.
fn faded(c: egui::Color32, factor: f32) -> egui::Color32 {
    let f = factor.clamp(0.0, 1.0);
    egui::Color32::from_rgba_premultiplied(
        (c.r() as f32 * f) as u8,
        (c.g() as f32 * f) as u8,
        (c.b() as f32 * f) as u8,
        (c.a() as f32 * f) as u8,
    )
}

impl eframe::App for TimerApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_poll.elapsed() >= Duration::from_secs(1) {
            self.cfg = Config::load();
            self.state = State::load();
            self.pomodoro_state = PomodoroState::load();
            self.last_poll = Instant::now();
        }

        let op = self.cfg.corner_timer_opacity;

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter();

                painter.rect_filled(rect, egui::Rounding::same(design::RADIUS_LG), faded(design::bg(), op));
                painter.rect_stroke(
                    rect,
                    egui::Rounding::same(design::RADIUS_LG),
                    egui::Stroke::new(1.0_f32, faded(design::divider(), op)),
                );

                let ring_center = rect.left_center() + egui::vec2(30.0, 0.0);
                let ring_r = 17.0;

                if !self.cfg.enabled {
                    painter.circle_stroke(
                        ring_center,
                        ring_r,
                        egui::Stroke::new(3.0_f32, faded(design::divider(), op)),
                    );
                    painter.text(
                        ring_center,
                        egui::Align2::CENTER_CENTER,
                        "⏸",
                        egui::FontId::proportional(13.0),
                        faded(design::neutral_600(), op),
                    );
                    painter.text(
                        rect.left_center() + egui::vec2(56.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        "Eye Break paused",
                        design::heading_font(15.0),
                        faded(design::text(), op),
                    );
                } else {
                    // Pomodoro mode replaces the plain interval scheduler
                    // entirely (see tray.rs's scheduler tick), so the
                    // countdown has to follow `PomodoroState` here too —
                    // otherwise this card keeps counting down against the
                    // General-tab interval, a schedule nothing is actually
                    // driving, and sits at 00:00 without a break ever
                    // showing once that unrelated countdown runs out.
                    let (remaining, elapsed_frac, label) = if self.cfg.pomodoro_enabled {
                        let pcfg = PomodoroConfig::from(&self.cfg);
                        let due = self.pomodoro_state.phase_due_epoch(&pcfg);
                        let remaining = due.saturating_sub(now_epoch());
                        let total = self.pomodoro_state.phase_duration_secs(&pcfg).max(1);
                        let elapsed_frac = 1.0 - (remaining as f32 / total as f32).clamp(0.0, 1.0);
                        (remaining, elapsed_frac, self.pomodoro_state.phase.label())
                    } else {
                        let interval = self.cfg.interval_secs.max(1);
                        let next = self.state.next_break_epoch(interval);
                        let remaining = next.saturating_sub(now_epoch());
                        let elapsed_frac = 1.0 - (remaining as f32 / interval as f32).clamp(0.0, 1.0);
                        (remaining, elapsed_frac, "NEXT BREAK IN")
                    };

                    painter.circle_stroke(
                        ring_center,
                        ring_r,
                        egui::Stroke::new(3.0_f32, faded(design::divider(), op)),
                    );
                    let start_deg = -90.0_f32;
                    let end_deg = start_deg + elapsed_frac * 360.0;
                    let steps = 32;
                    let pts: Vec<egui::Pos2> = (0..=steps)
                        .map(|i| {
                            let t = start_deg + (end_deg - start_deg) * (i as f32 / steps as f32);
                            let rad = t.to_radians();
                            ring_center + egui::vec2(ring_r * rad.cos(), ring_r * rad.sin())
                        })
                        .collect();
                    if pts.len() > 1 {
                        painter.add(egui::Shape::line(pts, egui::Stroke::new(3.0_f32, faded(design::accent(), op))));
                    }

                    let label_pos = rect.left_center() + egui::vec2(56.0, -9.0);
                    painter.text(
                        label_pos,
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(10.5),
                        faded(design::neutral_600(), op),
                    );
                    painter.text(
                        label_pos + egui::vec2(0.0, 17.0),
                        egui::Align2::LEFT_CENTER,
                        format!("{:02}:{:02}", remaining / 60, remaining % 60),
                        design::heading_font(19.0),
                        faded(design::text(), op),
                    );
                }
            });

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}
