use crate::config::Config;
use crate::monitors::primary_monitor;
use crate::state::{now_epoch, State};
use eframe::egui;
use std::time::{Duration, Instant};

const WIN_W: f32 = 210.0;
const WIN_H: f32 = 54.0;
const MARGIN: f32 = 16.0;

/// A small always-on-top pill in the corner of the primary monitor showing
/// "Next break in MM:SS". Runs as its own long-lived process so it doesn't
/// have to share an event loop with the GTK-driven tray icon.
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
        Box::new(|_cc| Ok(Box::new(TimerApp::new()))),
    )
}

struct TimerApp {
    last_poll: Instant,
    cfg: Config,
    state: State,
}

impl TimerApp {
    fn new() -> Self {
        Self {
            last_poll: Instant::now() - Duration::from_secs(10),
            cfg: Config::load(),
            state: State::load(),
        }
    }
}

impl eframe::App for TimerApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_poll.elapsed() >= Duration::from_secs(1) {
            self.cfg = Config::load();
            self.state = State::load();
            self.last_poll = Instant::now();
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let bg = egui::Color32::from_rgba_unmultiplied(20, 20, 20, 165);
                ui.painter()
                    .rect_filled(rect, egui::Rounding::same(10.0), bg);

                let text = if !self.cfg.enabled {
                    "Eye Break: paused".to_string()
                } else {
                    let next = self.state.next_break_epoch(self.cfg.interval_secs);
                    let remaining = next.saturating_sub(now_epoch());
                    format!(
                        "👁 Next break in {:02}:{:02}",
                        remaining / 60,
                        remaining % 60
                    )
                };

                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(15.0),
                    egui::Color32::from_rgb(235, 235, 235),
                );
            });

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}
