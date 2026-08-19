//! Visual chrome for the Settings window, ported from the "Classical" design
//! (warm/editorial palette, sidebar nav, radial gauges) at
//! claude.ai/design — see `Eye Break Settings.dc.html` in that project.
//!
//! This is deliberately separate from `theme.rs`: `theme::Theme` is a
//! *user-facing setting* that controls the break overlay/corner timer's
//! look, while this module is the Settings window's own fixed chrome (not
//! configurable — it's this app's "brand", the same way a website's own
//! chrome doesn't change even if it lets you pick a dark/light reading mode
//! for content).

use eframe::egui;

pub const BG: egui::Color32 = egui::Color32::from_rgb(0xf3, 0xf2, 0xf2);
pub const SURFACE: egui::Color32 = egui::Color32::from_rgb(0xea, 0xe9, 0xe9);
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(0x20, 0x1f, 0x1d);
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(0xb6, 0x82, 0x35);
pub const ACCENT_100: egui::Color32 = egui::Color32::from_rgb(0xff, 0xf3, 0xe4);
pub const ACCENT_300: egui::Color32 = egui::Color32::from_rgb(0xfa, 0xcb, 0x8d);
pub const ACCENT_500: egui::Color32 = egui::Color32::from_rgb(0xc2, 0x8d, 0x41);
pub const ACCENT_700: egui::Color32 = egui::Color32::from_rgb(0x7d, 0x54, 0x11);
pub const ACCENT_800: egui::Color32 = egui::Color32::from_rgb(0x5a, 0x3b, 0x0a);
pub const NEUTRAL_200: egui::Color32 = egui::Color32::from_rgb(0xea, 0xe7, 0xe7);
pub const NEUTRAL_600: egui::Color32 = egui::Color32::from_rgb(0x7d, 0x79, 0x79);
pub const DIVIDER: egui::Color32 = egui::Color32::from_rgba_premultiplied(0x20, 0x1f, 0x1d, 40);

pub const RADIUS_MD: f32 = 4.0;
/// Card/dialog-scale corner radius, for future use (the settings window's
/// own frame isn't rounded, since it's a native window, not the design's
/// mocked-up browser-chrome card) — kept for parity with the design tokens.
#[allow(dead_code)]
pub const RADIUS_LG: f32 = 7.0;

/// The design's serif display face (headings, big numbers), embedded from
/// Google Fonts (OFL-licensed) rather than relying on whatever serif the
/// system happens to have — the whole point of this design is a specific
/// editorial feel, which a generic system serif substitute wouldn't give.
pub const HEADING_FONT: &str = "cormorant-garamond";
/// The design's serif body/UI face — replaces egui's default sans-serif for
/// all normal text in this window, same reasoning as `HEADING_FONT`.
pub const BODY_FONT: &str = "lora";

/// Registers the Classical design's fonts and installs `install_fonts`'s
/// `BODY_FONT` as the default proportional family. Call this once, at
/// window creation (from the `run_native` creation closure) — NOT every
/// frame from `apply()`, since rebuilding the font atlas is expensive.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        HEADING_FONT.to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../assets/fonts/CormorantGaramond-SemiBold.ttf"
        )),
    );
    fonts.font_data.insert(
        BODY_FONT.to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Lora-Regular.ttf")),
    );

    fonts
        .families
        .entry(egui::FontFamily::Name(HEADING_FONT.into()))
        .or_default()
        .insert(0, HEADING_FONT.to_owned());

    // Lora replaces the default proportional family so ordinary labels,
    // buttons, and inputs pick it up automatically without every call site
    // needing to opt in.
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, BODY_FONT.to_owned());

    ctx.set_fonts(fonts);
}

/// `FontId` for the heading/display face at a given size.
pub fn heading_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name(HEADING_FONT.into()))
}

/// Applies the Classical chrome to an egui context — call once per frame
/// before drawing the settings window's panels.
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(TEXT);
    visuals.window_fill = BG;
    visuals.panel_fill = BG;
    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    visuals.widgets.inactive.weak_bg_fill = SURFACE;
    visuals.widgets.hovered.bg_fill = ACCENT_100;
    visuals.widgets.active.bg_fill = ACCENT_300;
    visuals.selection.bg_fill = ACCENT_300;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT_700);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, DIVIDER);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, DIVIDER);
    ctx.set_visuals(visuals);
}

/// A toggle switch matching the design's pill track + circular knob.
/// Returns `true` if it was clicked (caller flips its own bool and saves).
pub fn toggle_switch(ui: &mut egui::Ui, on: bool) -> egui::Response {
    const KNOB_R: f32 = 6.0;
    const KNOB_GAP: f32 = 2.0;
    let size = egui::vec2(34.0, 18.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let track_color = if on { ACCENT } else { egui::Color32::TRANSPARENT };
        let track_stroke = if on { ACCENT_700 } else { DIVIDER };
        painter.rect_filled(rect, 999.0, track_color);
        painter.rect_stroke(rect, 999.0, egui::Stroke::new(1.0_f32, track_stroke));
        let knob_x = if on {
            rect.right() - KNOB_R - KNOB_GAP
        } else {
            rect.left() + KNOB_R + KNOB_GAP
        };
        let knob_center = egui::pos2(knob_x, rect.center().y);
        let knob_color = if on { egui::Color32::WHITE } else { NEUTRAL_600 };
        painter.circle_filled(knob_center, KNOB_R, knob_color);
    }
    response
}

/// A pill-shaped chip button (interval presets, day-of-week picker). Returns
/// the click response; caller decides what selecting it means.
pub fn chip(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(12.5),
        TEXT,
    );
    let padding = egui::vec2(12.0, 5.0);
    let size = galley.size() + padding * 2.0;
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let (fill, stroke, text_color) = if active {
            (ACCENT_100, ACCENT, ACCENT_700)
        } else {
            (egui::Color32::TRANSPARENT, DIVIDER, TEXT)
        };
        ui.painter().rect_filled(rect, 999.0, fill);
        ui.painter().rect_stroke(rect, 999.0, egui::Stroke::new(1.0_f32, stroke));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.5),
            text_color,
        );
    }
    response
}

/// A segmented control (radio-button row rendered as one bordered strip),
/// matching the design's `.seg` component. Returns `Some(index)` of the
/// option clicked this frame.
pub fn segmented(ui: &mut egui::Ui, options: &[&str], current: usize) -> Option<usize> {
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        egui::Frame::none()
            .stroke(egui::Stroke::new(1.0_f32, DIVIDER))
            .rounding(RADIUS_MD)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, label) in options.iter().enumerate() {
                        let selected = i == current;
                        let (fill, text_color) = if selected {
                            (ACCENT_100, ACCENT_700)
                        } else {
                            (egui::Color32::TRANSPARENT, TEXT)
                        };
                        let resp = egui::Frame::none()
                            .fill(fill)
                            .inner_margin(egui::Margin::symmetric(12.0, 7.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(*label).size(13.0).color(text_color));
                            })
                            .response;
                        if ui.interact(resp.rect, resp.id.with("seg"), egui::Sense::click()).clicked() {
                            clicked = Some(i);
                        }
                        if i + 1 < options.len() {
                            ui.painter().vline(
                                ui.min_rect().right(),
                                ui.min_rect().y_range(),
                                egui::Stroke::new(1.0_f32, DIVIDER),
                            );
                        }
                    }
                });
            });
    });
    clicked
}

/// A radial progress gauge (track + accent arc), with two lines of centered
/// text — used for the reminder-interval dial.
pub fn radial_gauge(ui: &mut egui::Ui, diameter: f32, fraction: f32, big_text: &str, small_text: &str) {
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(diameter, diameter), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let center = rect.center();
    let radius = diameter / 2.0 - 5.0;
    let stroke_w = diameter * 0.067;

    painter.circle_stroke(center, radius, egui::Stroke::new(stroke_w, DIVIDER));

    let start_deg = -90.0_f32;
    let end_deg = start_deg + fraction.clamp(0.0, 0.999) * 360.0;
    let steps = 48;
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = start_deg + (end_deg - start_deg) * (i as f32 / steps as f32);
        let rad = t.to_radians();
        points.push(center + egui::vec2(radius * rad.cos(), radius * rad.sin()));
    }
    painter.add(egui::Shape::line(points, egui::Stroke::new(stroke_w, ACCENT)));

    painter.text(
        center + egui::vec2(0.0, -diameter * 0.06),
        egui::Align2::CENTER_CENTER,
        big_text,
        heading_font(diameter * 0.18),
        TEXT,
    );
    painter.text(
        center + egui::vec2(0.0, diameter * 0.11),
        egui::Align2::CENTER_CENTER,
        small_text,
        egui::FontId::proportional(diameter * 0.08),
        NEUTRAL_600,
    );
}

/// A 24-hour activity "wheel" — one wedge per hour, shaded by
/// `fractions[hour]` (0.0-1.0), matching the design's donut chart.
pub fn activity_wheel(ui: &mut egui::Ui, diameter: f32, fractions: &[f32; 24]) {
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(diameter, diameter), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let center = rect.center();
    let r_inner = diameter * 0.22;
    let r_outer_max = diameter * 0.47;

    for (h, frac) in fractions.iter().enumerate() {
        let start_deg = (h as f32 / 24.0) * 360.0 - 90.0;
        let end_deg = ((h as f32 + 1.0) / 24.0) * 360.0 - 90.0 - 1.2;
        let r_outer = r_inner + (r_outer_max - r_inner) * frac.clamp(0.02, 1.0);
        let color = lerp_color(NEUTRAL_200, ACCENT, *frac);

        let steps = 6;
        let mut pts = Vec::with_capacity(steps * 2 + 2);
        for i in 0..=steps {
            let t = start_deg + (end_deg - start_deg) * (i as f32 / steps as f32);
            let rad = t.to_radians();
            pts.push(center + egui::vec2(r_outer * rad.cos(), r_outer * rad.sin()));
        }
        for i in (0..=steps).rev() {
            let t = start_deg + (end_deg - start_deg) * (i as f32 / steps as f32);
            let rad = t.to_radians();
            pts.push(center + egui::vec2(r_inner * rad.cos(), r_inner * rad.sin()));
        }
        painter.add(egui::Shape::convex_polygon(pts, color, egui::Stroke::NONE));
    }

    painter.circle_filled(center, r_inner - 2.0, BG);
    painter.text(
        center + egui::vec2(0.0, -6.0),
        egui::Align2::CENTER_CENTER,
        "24h",
        egui::FontId::proportional(11.0),
        TEXT,
    );
    painter.text(
        center + egui::vec2(0.0, 7.0),
        egui::Align2::CENTER_CENTER,
        "activity",
        egui::FontId::proportional(9.0),
        NEUTRAL_600,
    );
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

/// A clock face with two hands (start/end), matching the workday-schedule
/// widget. Purely illustrative — the actual times are edited via inputs
/// alongside it, same as in the design.
pub fn clock_face(ui: &mut egui::Ui, diameter: f32, start_frac_of_12h: f32, end_frac_of_12h: f32) {
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(diameter, diameter), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let center = rect.center();
    let radius = diameter / 2.0 - 6.0;

    painter.circle_stroke(center, radius, egui::Stroke::new(diameter * 0.06, DIVIDER));

    let hand = |frac: f32, color: egui::Color32, length: f32| {
        let deg: f32 = frac * 360.0 - 90.0;
        let rad = deg.to_radians();
        let end = center + egui::vec2(length * rad.cos(), length * rad.sin());
        painter.line_segment([center, end], egui::Stroke::new(2.5_f32, color));
        painter.circle_filled(
            center + egui::vec2(radius * rad.cos(), radius * rad.sin()),
            4.5,
            color,
        );
    };
    hand(start_frac_of_12h, ACCENT, radius * 0.78);
    hand(end_frac_of_12h, ACCENT_800, radius * 0.78);
    painter.circle_filled(center, 3.0, TEXT);

    for (label, frac) in [("12", 0.0_f32), ("3", 0.25), ("6", 0.5), ("9", 0.75)] {
        let deg: f32 = frac * 360.0 - 90.0;
        let rad = deg.to_radians();
        let pos = center + egui::vec2((radius + 12.0) * rad.cos(), (radius + 12.0) * rad.sin());
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(9.0),
            NEUTRAL_600,
        );
    }
}

/// Which sidebar nav icon to draw — one per settings tab.
#[derive(Clone, Copy, PartialEq)]
pub enum NavIcon {
    General,
    Theme,
    Sound,
    Pomodoro,
    Schedule,
    Stats,
}

/// Draws one of the sidebar's line-art icons, geometry taken directly from
/// the design's inline SVGs (each defined on a 24x24 viewBox, 1.8px
/// stroke, round caps/joins) rather than approximated — so the nav matches
/// the design pixel-for-pixel rather than substituting emoji, which can't
/// be recolored to a single line weight/color the way these need to be.
pub fn nav_icon(painter: &egui::Painter, rect: egui::Rect, icon: NavIcon, color: egui::Color32) {
    // Map a design-space (0..24) point to screen space within `rect`.
    let s = rect.width() / 24.0;
    let p = |x: f32, y: f32| rect.left_top() + egui::vec2(x * s, y * s);
    let stroke = egui::Stroke::new((1.8 * s).max(1.0), color);
    let line = |painter: &egui::Painter, a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    };
    let circle = |painter: &egui::Painter, c: (f32, f32), r: f32| {
        painter.circle_stroke(p(c.0, c.1), r * s, stroke);
    };

    match icon {
        NavIcon::General => {
            // Three vertical sliders with handle dots.
            line(painter, (5.0, 21.0), (5.0, 10.0));
            line(painter, (5.0, 6.0), (5.0, 3.0));
            line(painter, (12.0, 21.0), (12.0, 14.0));
            line(painter, (12.0, 10.0), (12.0, 3.0));
            line(painter, (19.0, 21.0), (19.0, 16.0));
            line(painter, (19.0, 12.0), (19.0, 3.0));
            circle(painter, (5.0, 8.0), 2.0);
            circle(painter, (12.0, 12.0), 2.0);
            circle(painter, (19.0, 14.0), 2.0);
        }
        NavIcon::Theme => {
            // Painter's palette: a thumb-hole horseshoe (an open arc rather
            // than a full circle, so it actually reads as a palette and not
            // a wheel) plus three paint-well dots.
            let pts: Vec<egui::Pos2> = (0..=28)
                .map(|i| {
                    let t = 35.0 + (360.0 - 70.0) * (i as f32 / 28.0);
                    let rad = t.to_radians();
                    p(12.0, 12.5) + egui::vec2(8.5 * rad.cos(), 8.5 * rad.sin()) * s
                })
                .collect();
            painter.add(egui::Shape::line(pts, stroke));
            circle(painter, (7.5, 10.5), 1.1);
            circle(painter, (10.5, 6.8), 1.1);
            circle(painter, (15.2, 8.2), 1.1);
        }
        NavIcon::Sound => {
            // Bell: a proper dome (half-circle arc) flaring into a funnel
            // base, plus the clapper.
            let dome: Vec<egui::Pos2> = (0..=20)
                .map(|i| {
                    let t = 180.0 + 180.0 * (i as f32 / 20.0);
                    let rad = t.to_radians();
                    p(12.0, 9.0) + egui::vec2(5.5 * rad.cos(), 5.5 * rad.sin()) * s
                })
                .collect();
            painter.add(egui::Shape::line(dome, stroke));
            line(painter, (6.5, 9.0), (4.0, 15.0));
            line(painter, (17.5, 9.0), (20.0, 15.0));
            line(painter, (4.0, 15.0), (20.0, 15.0));
            let clapper: Vec<egui::Pos2> = (0..=10)
                .map(|i| {
                    let t = 180.0 + 180.0 * (i as f32 / 10.0);
                    let rad = t.to_radians();
                    p(12.0, 15.0) + egui::vec2(2.2 * rad.cos(), 2.2 * rad.sin()) * s
                })
                .collect();
            painter.add(egui::Shape::line(clapper, stroke));
        }
        NavIcon::Pomodoro => {
            // Timer: circle face, hand, top stem.
            circle(painter, (12.0, 13.0), 8.0);
            line(painter, (12.0, 13.0), (12.0, 9.0));
            line(painter, (9.0, 3.0), (15.0, 3.0));
        }
        NavIcon::Schedule => {
            // Calendar: rounded rect body, header rule, binder rings.
            let body = egui::Rect::from_min_max(p(3.0, 5.0), p(21.0, 21.0));
            painter.rect_stroke(body, (2.0 * s).max(1.0), stroke);
            line(painter, (3.0, 10.0), (21.0, 10.0));
            line(painter, (8.0, 3.0), (8.0, 7.0));
            line(painter, (16.0, 3.0), (16.0, 7.0));
        }
        NavIcon::Stats => {
            // Bar chart: three bars, ascending.
            line(painter, (4.0, 21.0), (4.0, 13.0));
            line(painter, (12.0, 21.0), (12.0, 6.0));
            line(painter, (20.0, 21.0), (20.0, 10.0));
        }
    }
}

/// The brand mark — an eye glyph (outline + pupil circle), matching the
/// design's sidebar header icon.
pub fn eye_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let s = rect.width() / 24.0;
    let p = |x: f32, y: f32| rect.left_top() + egui::vec2(x * s, y * s);
    let stroke = egui::Stroke::new((1.8 * s).max(1.0), color);
    // Outline: two shallow arcs meeting at the corners, approximated as a
    // stretched circle clipped isn't trivial in egui, so we draw it as an
    // explicit polyline through the same control points the SVG path
    // implies (a lens/eye shape).
    let pts: Vec<egui::Pos2> = [
        (2.0, 12.0),
        (6.0, 6.0),
        (12.0, 5.0),
        (18.0, 6.0),
        (22.0, 12.0),
        (18.0, 18.0),
        (12.0, 19.0),
        (6.0, 18.0),
        (2.0, 12.0),
    ]
    .iter()
    .map(|&(x, y)| p(x, y))
    .collect();
    painter.add(egui::Shape::line(pts, stroke));
    painter.circle_stroke(p(12.0, 12.0), 3.0 * s, stroke);
}
