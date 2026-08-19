//! The full configuration UI. Visual chrome ported from the "Classical"
//! design at claude.ai/design (`Eye Break Settings.dc.html`): a left-side
//! nav with six sections, warm/editorial palette, and painted gauges
//! (interval dial, workday clock face, 24h activity wheel) instead of plain
//! form controls where the design calls for them. See `design.rs` for the
//! shared tokens/widgets and `theme.rs` for the (separate, user-facing)
//! overlay/timer theme this window lets you pick.
//!
//! Launched as its own process (`eye-break --settings`), same as
//! `timer.rs`'s corner countdown, so it doesn't have to share an event loop
//! with the GTK-driven tray icon.

use crate::config::Config;
use crate::design;
use crate::sounds::SoundChoice;
use crate::stats::{hourly_activity_fractions, today_usage_secs, usage_last_n_days, UsageLog};
use crate::theme::Theme;
use crate::{autostart, updater};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub fn run_settings() -> eframe::Result<()> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size(egui::vec2(860.0, 620.0))
        .with_min_inner_size(egui::vec2(680.0, 480.0))
        .with_title("Eye Break — Settings");

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "eye-break-settings",
        options,
        Box::new(|cc| {
            // Font atlas rebuilds are expensive — install once here, not
            // every frame from design::apply().
            design::install_fonts(&cc.egui_ctx);
            Ok(Box::new(SettingsApp::new()))
        }),
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    General,
    Theme,
    Sound,
    Pomodoro,
    Schedule,
    Stats,
}

const TABS: &[(Tab, design::NavIcon, &str)] = &[
    (Tab::General, design::NavIcon::General, "General"),
    (Tab::Theme, design::NavIcon::Theme, "Theme"),
    (Tab::Sound, design::NavIcon::Sound, "Sound"),
    (Tab::Pomodoro, design::NavIcon::Pomodoro, "Pomodoro"),
    (Tab::Schedule, design::NavIcon::Schedule, "Schedule"),
    (Tab::Stats, design::NavIcon::Stats, "Stats"),
];

const INTERVAL_PRESETS_MIN: &[u64] = &[10, 15, 20, 30, 45, 60, 90, 120];

struct SettingsApp {
    cfg: Config,
    usage: UsageLog,
    autostart_enabled: bool,
    tab: Tab,
    stats_period_days: usize,
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
            tab: Tab::General,
            stats_period_days: 7,
            update_status: Arc::new(Mutex::new(None)),
        }
    }

    fn save_cfg(&self) {
        self.cfg.save();
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        design::apply(ctx, self.cfg.theme);

        egui::SidePanel::left("nav")
            .exact_width(210.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(design::surface())
                    .inner_margin(egui::Margin::symmetric(14.0, 22.0))
                    .stroke(egui::Stroke::NONE),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (eye_rect, _resp) =
                        ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                    design::eye_icon(ui.painter(), eye_rect, design::accent_700());
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Eye Break")
                                .font(design::heading_font(16.0))
                                .color(design::text()),
                        );
                        ui.label(
                            egui::RichText::new("20-20-20 reminders")
                                .size(10.5)
                                .color(design::neutral_600()),
                        );
                    });
                });
                ui.add_space(10.0);
                ui.painter().hline(
                    ui.min_rect().x_range(),
                    ui.min_rect().bottom(),
                    egui::Stroke::new(1.0_f32, design::divider()),
                );
                ui.add_space(10.0);

                for (tab, icon, label) in TABS {
                    let active = self.tab == *tab;
                    let row_size = egui::vec2(ui.available_width(), 32.0);
                    let (rect, response) = ui.allocate_exact_size(row_size, egui::Sense::click());
                    // Hover gets a faint tint distinct from the active fill,
                    // so the nav feels responsive to the cursor rather than
                    // only reacting on click — a small thing, but it's the
                    // difference between a static list and a "live" UI.
                    let (fill, text_color) = if active {
                        (design::accent_100(), design::accent_700())
                    } else if response.hovered() {
                        (design::neutral_200(), design::text())
                    } else {
                        (egui::Color32::TRANSPARENT, design::text())
                    };
                    if ui.is_rect_visible(rect) {
                        let painter = ui.painter();
                        painter.rect_filled(rect, design::RADIUS_MD, fill);
                        let icon_rect = egui::Rect::from_min_size(
                            rect.left_top() + egui::vec2(10.0, 8.0),
                            egui::vec2(16.0, 16.0),
                        );
                        design::nav_icon(painter, icon_rect, *icon, text_color);
                        painter.text(
                            rect.left_center() + egui::vec2(34.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::FontId::proportional(13.5),
                            text_color,
                        );
                    }
                    if response.clicked() {
                        self.tab = *tab;
                    }
                    ui.add_space(3.0);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(
                        egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .size(11.0)
                            .color(design::neutral_600()),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(design::bg())
                    .inner_margin(egui::Margin::symmetric(28.0, 24.0)),
            )
            .show(ctx, |ui| {
                design::card(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                        Tab::General => self.general_tab(ui),
                        Tab::Theme => self.theme_tab(ui),
                        Tab::Sound => self.sound_tab(ui),
                        Tab::Pomodoro => self.pomodoro_tab(ui),
                        Tab::Schedule => self.schedule_tab(ui),
                        Tab::Stats => self.stats_tab(ui),
                    });
                });
            });
    }
}

impl SettingsApp {
    fn heading(&self, ui: &mut egui::Ui, title: &str, subtitle: &str) {
        ui.label(
            egui::RichText::new(title)
                .font(design::heading_font(24.0))
                .color(design::text()),
        );
        ui.add_space(2.0);
        ui.label(egui::RichText::new(subtitle).size(12.5).color(design::neutral_600()));
        ui.add_space(20.0);
    }

    fn toggle_row(&mut self, ui: &mut egui::Ui, label: &str, sublabel: &str, value: bool) -> bool {
        // Note: deliberately not `ui.with_layout(right_to_left(...))` for the
        // toggle side — without an explicit size, that expands to fill the
        // row's *entire remaining height* (a well-known egui gotcha), which
        // is what was blowing up the switches and the gap between rows.
        // Right-aligning by padding with a fixed-size spacer keeps the row's
        // height determined only by its actual content.
        const TOGGLE_W: f32 = 34.0;
        let mut clicked = false;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(label).size(14.0).color(design::text()));
                ui.label(egui::RichText::new(sublabel).size(12.0).color(design::neutral_600()));
            });
            let remaining = ui.available_width();
            if remaining > TOGGLE_W {
                ui.add_space(remaining - TOGGLE_W);
            }
            if design::toggle_switch(ui, value).clicked() {
                clicked = true;
            }
        });
        clicked
    }

    fn general_tab(&mut self, ui: &mut egui::Ui) {
        self.heading(ui, "General", "Core reminder behavior — how often, how long, and what it says.");

        if self.toggle_row(ui, "Enabled", "Turn reminders on or off entirely", self.cfg.enabled) {
            self.cfg.enabled = !self.cfg.enabled;
            self.save_cfg();
        }
        ui.add_space(14.0);
        if self.toggle_row(
            ui,
            "Show corner countdown",
            "A small always-on-top timer to the next break",
            self.cfg.show_timer,
        ) {
            self.cfg.show_timer = !self.cfg.show_timer;
            self.save_cfg();
        }
        if self.cfg.show_timer {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Opacity:").size(12.0).color(design::neutral_600()));
                if ui
                    .add(
                        egui::Slider::new(&mut self.cfg.corner_timer_opacity, 0.1..=1.0)
                            .show_value(false),
                    )
                    .changed()
                {
                    self.save_cfg();
                }
                ui.label(
                    egui::RichText::new(format!("{}%", (self.cfg.corner_timer_opacity * 100.0).round() as i32))
                        .size(12.0)
                        .color(design::neutral_600()),
                );
            });
        }

        ui.add_space(16.0);
        ui.painter().hline(ui.min_rect().x_range(), ui.min_rect().bottom(), egui::Stroke::new(1.0_f32, design::divider()));
        ui.add_space(16.0);

        ui.horizontal(|ui| {
            let frac = (self.cfg.interval_secs as f32 / 60.0 / 60.0).min(1.0);
            design::radial_gauge(ui, 120.0, frac, &(self.cfg.interval_secs / 60).to_string(), "minutes");
            ui.add_space(28.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Reminder interval").size(14.0).color(design::text()));
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    for &min in INTERVAL_PRESETS_MIN {
                        let active = self.cfg.interval_secs == min * 60;
                        if design::chip(ui, &format!("{min} min"), active).clicked() {
                            self.cfg.interval_secs = min * 60;
                            self.save_cfg();
                        }
                    }
                });
            });
        });

        ui.add_space(20.0);
        ui.columns(2, |cols| {
            cols[0].label(egui::RichText::new("Break duration (sec)").size(12.0).color(design::neutral_600()));
            if cols[0].add(egui::DragValue::new(&mut self.cfg.display_secs).range(1..=120)).changed() {
                self.save_cfg();
            }
            cols[1].label(egui::RichText::new("Snooze length (min)").size(12.0).color(design::neutral_600()));
            let mut snooze_min = self.cfg.snooze_secs / 60;
            if cols[1].add(egui::DragValue::new(&mut snooze_min).range(1..=180)).changed() {
                self.cfg.snooze_secs = snooze_min * 60;
                self.save_cfg();
            }
        });

        ui.add_space(16.0);
        ui.label(egui::RichText::new("Reminder text").size(12.0).color(design::neutral_600()));
        if ui.text_edit_singleline(&mut self.cfg.reminder_text).changed() {
            self.save_cfg();
        }

        ui.add_space(24.0);
        ui.painter().hline(ui.min_rect().x_range(), ui.min_rect().bottom(), egui::Stroke::new(1.0_f32, design::divider()));
        ui.add_space(16.0);
        ui.label(egui::RichText::new("Startup & updates").size(14.0).color(design::text()));
        ui.add_space(8.0);
        let mut autostart_on = self.autostart_enabled;
        if self.toggle_row(ui, "Run on startup", "Launch automatically when you log in", autostart_on) {
            autostart_on = !autostart_on;
            let _ = autostart::set_enabled(autostart_on);
            self.autostart_enabled = autostart::is_enabled();
        }
        ui.add_space(10.0);
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
            ui.label(egui::RichText::new(status).size(12.0).color(design::neutral_600()));
        }
    }

    fn theme_tab(&mut self, ui: &mut egui::Ui) {
        self.heading(ui, "Theme", "Visual theme applied to the break overlay.");

        egui::Grid::new("theme-grid").spacing([14.0, 14.0]).show(ui, |ui| {
            for (i, &t) in Theme::all().iter().enumerate() {
                let active = self.cfg.theme == t;
                let (bg, _panel, accent, text) = crate::theme::palette(t);
                let (fill, stroke) = if active {
                    (design::accent_100(), egui::Stroke::new(1.5_f32, design::accent()))
                } else {
                    (egui::Color32::TRANSPARENT, egui::Stroke::new(1.0_f32, design::divider()))
                };
                let resp = egui::Frame::none()
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(design::RADIUS_MD)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_width(140.0);
                        ui.horizontal(|ui| {
                            for c in [bg, accent, text] {
                                let (r, _resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                                ui.painter().rect_filled(r, 3.0, c);
                                ui.painter().rect_stroke(r, 3.0, egui::Stroke::new(1.0_f32, design::divider()));
                            }
                        });
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(t.label()).size(13.0).color(design::text()));
                    })
                    .response;
                if ui.interact(resp.rect, resp.id.with("theme-card"), egui::Sense::click()).clicked() {
                    self.cfg.theme = t;
                    self.save_cfg();
                }
                if (i + 1) % 3 == 0 {
                    ui.end_row();
                }
            }
        });
    }

    fn sound_tab(&mut self, ui: &mut egui::Ui) {
        self.heading(ui, "Sound", "What plays when a break overlay appears.");

        let options = ["None", "Chime", "Bell", "Custom"];
        let current = match self.cfg.sound {
            SoundChoice::None => 0,
            SoundChoice::Chime => 1,
            SoundChoice::Bell => 2,
            SoundChoice::Custom(_) => 3,
        };
        if let Some(i) = design::segmented(ui, &options, current) {
            self.cfg.sound = match i {
                0 => SoundChoice::None,
                1 => SoundChoice::Chime,
                2 => SoundChoice::Bell,
                _ => SoundChoice::Custom(String::new()),
            };
            self.save_cfg();
        }

        ui.add_space(16.0);
        if ui.button("🔔  Preview sound").clicked() {
            crate::sounds::play(&self.cfg.sound);
        }

        if let SoundChoice::Custom(_) = &self.cfg.sound {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("Custom sound path").size(12.0).color(design::neutral_600()));
            let mut path = match &self.cfg.sound {
                SoundChoice::Custom(p) => p.clone(),
                _ => String::new(),
            };
            let resp = ui.add(
                egui::TextEdit::singleline(&mut path).hint_text("/home/you/sounds/chime.ogg"),
            );
            if resp.changed() {
                self.cfg.sound = SoundChoice::Custom(path);
                self.save_cfg();
            }
        }
    }

    fn pomodoro_tab(&mut self, ui: &mut egui::Ui) {
        self.heading(ui, "Pomodoro", "An alternate scheduler that alternates work and break phases.");

        if self.toggle_row(
            ui,
            "Enable Pomodoro mode",
            "Takes over scheduling from the plain interval in General",
            self.cfg.pomodoro_enabled,
        ) {
            self.cfg.pomodoro_enabled = !self.cfg.pomodoro_enabled;
            self.save_cfg();
        }

        ui.add_space(20.0);
        ui.columns(2, |cols| {
            cols[0].label(egui::RichText::new("Work (min)").size(12.0).color(design::neutral_600()));
            if cols[0].add(egui::DragValue::new(&mut self.cfg.pomodoro_work_mins).range(1..=120)).changed() {
                self.save_cfg();
            }
            cols[1].label(egui::RichText::new("Short break (min)").size(12.0).color(design::neutral_600()));
            if cols[1].add(egui::DragValue::new(&mut self.cfg.pomodoro_short_break_mins).range(1..=60)).changed() {
                self.save_cfg();
            }
        });
        ui.add_space(10.0);
        ui.columns(2, |cols| {
            cols[0].label(egui::RichText::new("Long break (min)").size(12.0).color(design::neutral_600()));
            if cols[0].add(egui::DragValue::new(&mut self.cfg.pomodoro_long_break_mins).range(1..=120)).changed() {
                self.save_cfg();
            }
            cols[1].label(egui::RichText::new("Cycles before long break").size(12.0).color(design::neutral_600()));
            if cols[1].add(egui::DragValue::new(&mut self.cfg.pomodoro_cycles_before_long_break).range(1..=12)).changed() {
                self.save_cfg();
            }
        });

        ui.add_space(24.0);
        ui.label(egui::RichText::new("Cycle preview").size(12.0).color(design::neutral_600()));
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let cycles = self.cfg.pomodoro_cycles_before_long_break.max(1);
            for i in 0..cycles {
                tag(ui, "Work", design::accent_100(), design::accent_800());
                let is_last = i + 1 == cycles;
                tag(ui, if is_last { "Long" } else { "Short" }, design::neutral_200(), design::text());
            }
        });
    }

    fn schedule_tab(&mut self, ui: &mut egui::Ui) {
        self.heading(ui, "Workday schedule", "Restrict reminders to certain hours and days.");

        if self.toggle_row(
            ui,
            "Only remind during workday hours",
            "Outside this window, breaks stay silent",
            self.cfg.workday_enabled,
        ) {
            self.cfg.workday_enabled = !self.cfg.workday_enabled;
            self.save_cfg();
        }

        ui.add_space(20.0);
        ui.horizontal(|ui| {
            let start_frac = (self.cfg.workday_start_hour % 12) as f32 / 12.0
                + self.cfg.workday_start_minute as f32 / 720.0;
            let end_frac = (self.cfg.workday_end_hour % 12) as f32 / 12.0
                + self.cfg.workday_end_minute as f32 / 720.0;
            design::clock_face(ui, 140.0, start_frac, end_frac);
            ui.add_space(32.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(design::accent(), "●");
                    ui.label(egui::RichText::new("Start").size(12.5).color(design::text()));
                });
                let mut changed = false;
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut self.cfg.workday_start_hour).range(0..=23)).changed();
                    ui.label(":");
                    changed |= ui.add(egui::DragValue::new(&mut self.cfg.workday_start_minute).range(0..=59)).changed();
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.colored_label(design::accent_800(), "●");
                    ui.label(egui::RichText::new("End").size(12.5).color(design::text()));
                });
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut self.cfg.workday_end_hour).range(0..=23)).changed();
                    ui.label(":");
                    changed |= ui.add(egui::DragValue::new(&mut self.cfg.workday_end_minute).range(0..=59)).changed();
                });
                if changed {
                    self.save_cfg();
                }
            });
        });

        ui.add_space(20.0);
        ui.label(egui::RichText::new("Active days").size(12.0).color(design::neutral_600()));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
            for (i, name) in day_names.iter().enumerate() {
                if design::chip(ui, name, self.cfg.workday_days[i]).clicked() {
                    self.cfg.workday_days[i] = !self.cfg.workday_days[i];
                    self.save_cfg();
                }
            }
        });

        ui.add_space(24.0);
        ui.painter().hline(ui.min_rect().x_range(), ui.min_rect().bottom(), egui::Stroke::new(1.0_f32, design::divider()));
        ui.add_space(16.0);
        ui.label(egui::RichText::new("Smart pausing").size(14.0).color(design::text()));
        ui.add_space(8.0);
        if self.toggle_row(
            ui,
            "Pause while idle",
            "No keyboard/mouse input for a while means nobody's there",
            self.cfg.idle_pause_enabled,
        ) {
            self.cfg.idle_pause_enabled = !self.cfg.idle_pause_enabled;
            self.save_cfg();
        }
        if self.cfg.idle_pause_enabled {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Idle after (min):").size(12.0).color(design::neutral_600()));
                if ui.add(egui::DragValue::new(&mut self.cfg.idle_pause_after_mins).range(1..=60)).changed() {
                    self.save_cfg();
                }
            });
        }
        ui.add_space(10.0);
        if self.toggle_row(
            ui,
            "Don't interrupt fullscreen apps",
            "Calls, presentations, and video stay uninterrupted",
            self.cfg.fullscreen_pause_enabled,
        ) {
            self.cfg.fullscreen_pause_enabled = !self.cfg.fullscreen_pause_enabled;
            self.save_cfg();
        }
    }

    fn stats_tab(&mut self, ui: &mut egui::Ui) {
        // See the note in `toggle_row`: an unconstrained
        // `with_layout(right_to_left(...))` here would blow this row up to
        // the tab's full remaining height, so we allocate a fixed-size
        // right-hand region for the segmented control instead.
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Usage stats")
                    .font(design::heading_font(24.0))
                    .color(design::text()),
            );
            ui.allocate_ui_with_layout(
                egui::vec2(160.0, 24.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    let options = ["90d", "30d", "7d"];
                    let current = match self.stats_period_days {
                        90 => 0,
                        30 => 1,
                        _ => 2,
                    };
                    if let Some(i) = design::segmented(ui, &options, current) {
                        self.stats_period_days = [90, 30, 7][i];
                    }
                },
            );
        });
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new("Device activity, so you can see how much screen time these breaks are covering.")
                .size(12.5)
                .color(design::neutral_600()),
        );
        ui.add_space(18.0);

        let today = today_usage_secs(&self.usage);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{}h {}m", today / 3600, (today % 3600) / 60))
                    .font(design::heading_font(30.0))
                    .color(design::text()),
            );
            ui.label(egui::RichText::new("today so far").size(12.0).color(design::neutral_600()));
        });

        ui.add_space(14.0);
        usage_bar_chart(ui, &usage_last_n_days(&self.usage, self.stats_period_days));

        ui.add_space(24.0);
        ui.painter().hline(ui.min_rect().x_range(), ui.min_rect().bottom(), egui::Stroke::new(1.0_f32, design::divider()));
        ui.add_space(16.0);
        ui.label(egui::RichText::new("Most active hours").size(14.0).color(design::text()));
        ui.label(
            egui::RichText::new("Darker wedges mean more device activity in that hour, over the selected period.")
                .size(12.0)
                .color(design::neutral_600()),
        );
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let fractions = hourly_activity_fractions(&self.usage, self.stats_period_days);
            design::activity_wheel(ui, 160.0, &fractions);
        });
    }
}

fn tag(ui: &mut egui::Ui, label: &str, bg: egui::Color32, text_color: egui::Color32) {
    egui::Frame::none()
        .fill(bg)
        .rounding(3.0)
        .inner_margin(egui::Margin::symmetric(10.0, 3.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(label).size(11.0).color(text_color));
        });
}

/// A simple daily-usage bar chart, painted by hand — colors come from the
/// Classical design tokens so it matches the rest of the window.
fn usage_bar_chart(ui: &mut egui::Ui, days: &[crate::stats::DailyUsage]) {
    if days.is_empty() {
        ui.label(egui::RichText::new("No usage recorded yet.").color(design::neutral_600()));
        return;
    }

    const CHART_HEIGHT: f32 = 130.0;
    const LABEL_HEIGHT: f32 = 18.0;

    let desired_size = egui::vec2(ui.available_width(), CHART_HEIGHT + LABEL_HEIGHT);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    let max_secs = days.iter().map(|d| d.active_secs).max().unwrap_or(1).max(1);
    let bar_count = days.len() as f32;
    let gap = if bar_count > 20.0 { 2.0 } else { 6.0 };
    let bar_w = (rect.width() - gap * (bar_count - 1.0).max(0.0)) / bar_count;

    for (i, day) in days.iter().enumerate() {
        let x0 = rect.left() + i as f32 * (bar_w + gap);
        let x1 = x0 + bar_w;

        let bar_h = CHART_HEIGHT * (day.active_secs as f32 / max_secs as f32).clamp(0.02, 1.0);
        let bar_rect = egui::Rect::from_min_max(
            egui::pos2(x0, rect.top() + CHART_HEIGHT - bar_h),
            egui::pos2(x1, rect.top() + CHART_HEIGHT),
        );
        painter.rect_filled(bar_rect, 2.0, design::accent_500());

        if days.len() <= 14 {
            let short_label = day.date.get(5..).unwrap_or(&day.date);
            painter.text(
                egui::pos2(x0 + bar_w / 2.0, rect.top() + CHART_HEIGHT + LABEL_HEIGHT / 2.0),
                egui::Align2::CENTER_CENTER,
                short_label,
                egui::FontId::proportional(10.0),
                design::neutral_600(),
            );
        }

        let hover_response = ui.interact(bar_rect, ui.id().with(("usage-bar", i)), egui::Sense::hover());
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
    painter.hline(rect.x_range(), rect.top() + CHART_HEIGHT, egui::Stroke::new(1.0_f32, design::divider()));
}
