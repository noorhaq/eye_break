//! The full configuration UI: a single egui window exposing every setting
//! added across the theme/sounds/pomodoro/stats/reminder-text features,
//! plus run-on-startup and manual update checks. Launched as its own
//! process (`eye-break --settings`) so it doesn't have to share an event
//! loop with the GTK-driven tray icon, same as timer.rs's corner countdown.

use crate::config::Config;
use crate::sounds::SoundChoice;
use crate::stats::{today_usage_secs, usage_last_n_days, DailyUsage, UsageLog};
use crate::theme::{self, Theme};
use crate::{autostart, updater};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub fn run_settings() -> eframe::Result<()> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size(egui::vec2(480.0, 640.0))
        .with_min_inner_size(egui::vec2(420.0, 480.0))
        .with_title("Eye Break — Settings");

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "eye-break-settings",
        options,
        Box::new(|_cc| Ok(Box::new(SettingsApp::new()))),
    )
}

struct SettingsApp {
    cfg: Config,
    usage: UsageLog,
    autostart_enabled: bool,
    /// Shared with the background thread spawned by "Check for updates" so
    /// the (potentially several-second) network call never blocks the UI
    /// thread. `Some("Checking…")` while in flight, replaced with the result
    /// when the thread finishes.
    update_status: Arc<Mutex<Option<String>>>,
}

impl SettingsApp {
    fn new() -> Self {
        Self {
            cfg: Config::load(),
            usage: UsageLog::load(),
            autostart_enabled: autostart::is_enabled(),
            update_status: Arc::new(Mutex::new(None)),
        }
    }

    fn save_cfg(&self) {
        self.cfg.save();
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::apply(ctx, self.cfg.theme);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(20.0, 16.0)))
            .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("👁").size(28.0));
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("Eye Break")
                            .size(24.0)
                            .strong(),
                    );
                });
                ui.label(
                    egui::RichText::new("Settings")
                        .size(13.0)
                        .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(12.0);

                egui::CollapsingHeader::new("⚙  General").default_open(true).show(ui, |ui| {
                    let mut changed = false;
                    changed |= ui.checkbox(&mut self.cfg.enabled, "Enabled").changed();
                    changed |= ui
                        .checkbox(&mut self.cfg.show_timer, "Show corner countdown")
                        .changed();

                    ui.horizontal(|ui| {
                        ui.label("Reminder interval (min):");
                        let mut mins = self.cfg.interval_secs / 60;
                        if ui.add(egui::DragValue::new(&mut mins).range(1..=240)).changed() {
                            self.cfg.interval_secs = mins * 60;
                            changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Break duration (sec):");
                        changed |= ui
                            .add(egui::DragValue::new(&mut self.cfg.display_secs).range(1..=120))
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Snooze length (min):");
                        let mut mins = self.cfg.snooze_secs / 60;
                        if ui.add(egui::DragValue::new(&mut mins).range(1..=180)).changed() {
                            self.cfg.snooze_secs = mins * 60;
                            changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Reminder text:");
                        changed |= ui.text_edit_singleline(&mut self.cfg.reminder_text).changed();
                    });

                    if changed {
                        self.save_cfg();
                    }
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                egui::CollapsingHeader::new("🎨  Theme").show(ui, |ui| {
                    let mut changed = false;
                    egui::ComboBox::from_label("Theme")
                        .selected_text(self.cfg.theme.label())
                        .show_ui(ui, |ui| {
                            for &t in Theme::all() {
                                if ui
                                    .selectable_value(&mut self.cfg.theme, t, t.label())
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });
                    if changed {
                        self.save_cfg();
                    }
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                egui::CollapsingHeader::new("🔔  Sound").show(ui, |ui| {
                    let mut changed = false;
                    let current_label = match &self.cfg.sound {
                        SoundChoice::None => "None",
                        SoundChoice::Chime => "Chime",
                        SoundChoice::Bell => "Bell",
                        SoundChoice::Custom(_) => "Custom",
                    };
                    egui::ComboBox::from_label("Notification sound")
                        .selected_text(current_label)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(current_label == "None", "None")
                                .clicked()
                            {
                                self.cfg.sound = SoundChoice::None;
                                changed = true;
                            }
                            if ui
                                .selectable_label(current_label == "Chime", "Chime")
                                .clicked()
                            {
                                self.cfg.sound = SoundChoice::Chime;
                                changed = true;
                            }
                            if ui
                                .selectable_label(current_label == "Bell", "Bell")
                                .clicked()
                            {
                                self.cfg.sound = SoundChoice::Bell;
                                changed = true;
                            }
                        });

                    if ui.button("Preview sound").clicked() {
                        crate::sounds::play(&self.cfg.sound);
                    }

                    ui.horizontal(|ui| {
                        ui.label("Custom sound path:");
                        let mut path = match &self.cfg.sound {
                            SoundChoice::Custom(p) => p.clone(),
                            _ => String::new(),
                        };
                        if ui.text_edit_singleline(&mut path).changed() && !path.is_empty() {
                            self.cfg.sound = SoundChoice::Custom(path);
                            changed = true;
                        }
                    });

                    if changed {
                        self.save_cfg();
                    }
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                egui::CollapsingHeader::new("🍅  Pomodoro").show(ui, |ui| {
                    let mut changed = false;
                    changed |= ui
                        .checkbox(&mut self.cfg.pomodoro_enabled, "Enable Pomodoro mode")
                        .changed();
                    ui.add_enabled_ui(self.cfg.pomodoro_enabled, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Work (min):");
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut self.cfg.pomodoro_work_mins)
                                        .range(1..=120),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label("Short break (min):");
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut self.cfg.pomodoro_short_break_mins)
                                        .range(1..=60),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label("Long break (min):");
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut self.cfg.pomodoro_long_break_mins)
                                        .range(1..=120),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label("Cycles before long break:");
                            changed |= ui
                                .add(egui::DragValue::new(
                                    &mut self.cfg.pomodoro_cycles_before_long_break,
                                ))
                                .changed();
                        });
                    });
                    if changed {
                        self.save_cfg();
                    }
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                egui::CollapsingHeader::new("🗓  Workday schedule").show(ui, |ui| {
                    let mut changed = false;
                    changed |= ui
                        .checkbox(&mut self.cfg.workday_enabled, "Only remind during workday hours")
                        .changed();
                    ui.add_enabled_ui(self.cfg.workday_enabled, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Start hour (0-23):");
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut self.cfg.workday_start_hour)
                                        .range(0..=23),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label("End hour (0-23):");
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut self.cfg.workday_end_hour)
                                        .range(0..=23),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            let day_names =
                                ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
                            for (i, name) in day_names.iter().enumerate() {
                                changed |=
                                    ui.checkbox(&mut self.cfg.workday_days[i], *name).changed();
                            }
                        });
                    });
                    if changed {
                        self.save_cfg();
                    }
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                egui::CollapsingHeader::new("📊  Usage stats").default_open(true).show(ui, |ui| {
                    let today = today_usage_secs(&self.usage);
                    ui.label(format!(
                        "Today's device usage: {}h {}m",
                        today / 3600,
                        (today % 3600) / 60
                    ));
                    ui.add_space(8.0);
                    usage_bar_chart(ui, &usage_last_n_days(&self.usage, 7));
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                egui::CollapsingHeader::new("🚀  Startup & updates").show(ui, |ui| {
                    let mut enabled = self.autostart_enabled;
                    if ui.checkbox(&mut enabled, "Run on startup").changed() {
                        let _ = autostart::set_enabled(enabled);
                        self.autostart_enabled = autostart::is_enabled();
                    }
                    if ui.button("Check for updates").clicked() {
                        *self.update_status.lock().unwrap() = Some("Checking…".to_string());
                        let status = self.update_status.clone();
                        updater::check_for_update_async(env!("CARGO_PKG_VERSION"), move |result| {
                            let text = match result {
                                Some(v) => format!("Update available: {v}"),
                                None => "You're up to date.".to_string(),
                            };
                            *status.lock().unwrap() = Some(text);
                        });
                    }
                    if let Some(status) = self.update_status.lock().unwrap().as_ref() {
                        ui.label(status);
                    }
                });
            });
        });

        // Usage stats/tick updates aren't relevant to this window's own
        // lifetime — it's a settings dialog, not the long-running scheduler.
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}

/// A simple daily-usage bar chart, painted by hand (no charting crate) so
/// it's trivial to restyle once a real design lands — colors come straight
/// from `ui.visuals()`, and the whole thing is one self-contained function.
/// Hovering a bar shows the exact date/duration in a tooltip.
fn usage_bar_chart(ui: &mut egui::Ui, days: &[DailyUsage]) {
    if days.is_empty() {
        ui.label("No usage recorded yet.");
        return;
    }

    const CHART_HEIGHT: f32 = 140.0;
    const BAR_GAP: f32 = 10.0;
    const LABEL_HEIGHT: f32 = 18.0;

    let desired_size = egui::vec2(ui.available_width(), CHART_HEIGHT + LABEL_HEIGHT);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    let max_secs = days.iter().map(|d| d.active_secs).max().unwrap_or(1).max(1);
    let bar_count = days.len() as f32;
    let bar_w = (rect.width() - BAR_GAP * (bar_count - 1.0).max(0.0)) / bar_count;

    let accent = ui.visuals().selection.bg_fill;
    let track = ui.visuals().widgets.noninteractive.bg_fill;
    let text_color = ui.visuals().text_color();

    for (i, day) in days.iter().enumerate() {
        let x0 = rect.left() + i as f32 * (bar_w + BAR_GAP);
        let x1 = x0 + bar_w;

        let bar_h = CHART_HEIGHT * (day.active_secs as f32 / max_secs as f32).clamp(0.02, 1.0);
        let track_rect =
            egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.top() + CHART_HEIGHT));
        let bar_rect = egui::Rect::from_min_max(
            egui::pos2(x0, rect.top() + CHART_HEIGHT - bar_h),
            egui::pos2(x1, rect.top() + CHART_HEIGHT),
        );

        painter.rect_filled(track_rect, 3.0, track);
        painter.rect_filled(bar_rect, 3.0, accent);

        // Day label (short weekday-ish suffix of the ISO date) under the bar.
        let short_label = day.date.get(5..).unwrap_or(&day.date); // "MM-DD"
        painter.text(
            egui::pos2(x0 + bar_w / 2.0, rect.top() + CHART_HEIGHT + LABEL_HEIGHT / 2.0),
            egui::Align2::CENTER_CENTER,
            short_label,
            egui::FontId::proportional(11.0),
            text_color,
        );

        // Hover tooltip with the exact value.
        let hover_response = ui.interact(
            track_rect,
            ui.id().with(("usage-bar", i)),
            egui::Sense::hover(),
        );
        if hover_response.hovered() {
            egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), ui.id().with("usage-tip"), |ui| {
                ui.label(format!(
                    "{}: {}h {}m",
                    day.date,
                    day.active_secs / 3600,
                    (day.active_secs % 3600) / 60
                ));
            });
        }
    }
}
