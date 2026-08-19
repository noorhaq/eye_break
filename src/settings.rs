//! The full configuration UI: a single egui window exposing every setting
//! added across the theme/sounds/pomodoro/stats/reminder-text features,
//! plus run-on-startup and manual update checks. Launched as its own
//! process (`eye-break --settings`) so it doesn't have to share an event
//! loop with the GTK-driven tray icon, same as timer.rs's corner countdown.

use crate::config::Config;
use crate::sounds::SoundChoice;
use crate::stats::{today_usage_secs, usage_last_n_days, UsageLog};
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

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Eye Break Settings");
                ui.add_space(8.0);

                ui.collapsing("General", |ui| {
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

                ui.collapsing("Theme", |ui| {
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

                ui.collapsing("Sound", |ui| {
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

                ui.collapsing("Pomodoro", |ui| {
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

                ui.collapsing("Workday schedule", |ui| {
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

                ui.collapsing("Usage stats", |ui| {
                    let today = today_usage_secs(&self.usage);
                    ui.label(format!(
                        "Today's device usage: {}h {}m",
                        today / 3600,
                        (today % 3600) / 60
                    ));
                    ui.add_space(4.0);
                    ui.label("Last 7 days:");
                    for day in usage_last_n_days(&self.usage, 7) {
                        ui.label(format!(
                            "  {}: {}h {}m",
                            day.date,
                            day.active_secs / 3600,
                            (day.active_secs % 3600) / 60
                        ));
                    }
                });

                ui.collapsing("Startup & updates", |ui| {
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
