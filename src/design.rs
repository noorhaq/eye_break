//! Visual chrome for the Settings window, ported from the "Classical" design
//! (warm/editorial layout — sidebar nav, radial gauges, card-elevated
//! content) at claude.ai/design — see `Eye Break Settings.dc.html` in that
//! project.
//!
//! Unlike the layout, the *colors* are not fixed: the Settings window now
//! follows the same user-selectable `Theme` as the break overlay/corner
//! timer, so picking e.g. "Nord" in Settings recolors Settings itself too.
//! `palette_for()` below derives a complete, contrast-aware, properly
//! saturated 12-color palette procedurally from each theme's 4 base colors
//! (`theme::palette`) — tints/shades are generated relative to that theme's
//! own darkness, not just the original warm palette's fixed offsets slapped
//! onto different hues, which would produce muddy or illegible results for
//! e.g. the dark themes.
//!
//! The corner timer is the one exception: it never calls `apply()`, so its
//! thread-local palette stays at `CLASSICAL` — it deliberately keeps its own
//! fixed look regardless of the selected Theme (a prior, explicit decision;
//! see `timer.rs`).

use crate::theme::Theme;
use eframe::egui;
use std::cell::Cell;

/// A complete, ready-to-paint color palette: page/surface/text base colors
/// plus a full accent ramp and the couple of derived neutrals/dividers the
/// hand-painted widgets below need. Everything in this module reads colors
/// through the free functions further down (`bg()`, `accent()`, ...) rather
/// than fields on a passed-around struct, so every widget function keeps its
/// original simple signature — the active palette is frame-global state, set
/// once by `apply()`.
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: egui::Color32,
    pub surface: egui::Color32,
    pub text: egui::Color32,
    pub accent: egui::Color32,
    pub accent_100: egui::Color32,
    pub accent_300: egui::Color32,
    pub accent_500: egui::Color32,
    pub accent_700: egui::Color32,
    pub accent_800: egui::Color32,
    pub neutral_200: egui::Color32,
    pub neutral_600: egui::Color32,
    pub divider: egui::Color32,
    pub is_dark: bool,
}

/// The original hand-tuned warm/editorial palette this design shipped with.
/// Used by the corner timer (which keeps a fixed look) and as the initial
/// thread-local default before `apply()` has run.
const CLASSICAL: Palette = Palette {
    bg: egui::Color32::from_rgb(0xf3, 0xf2, 0xf2),
    surface: egui::Color32::from_rgb(0xea, 0xe9, 0xe9),
    text: egui::Color32::from_rgb(0x20, 0x1f, 0x1d),
    accent: egui::Color32::from_rgb(0xb6, 0x82, 0x35),
    accent_100: egui::Color32::from_rgb(0xff, 0xf3, 0xe4),
    accent_300: egui::Color32::from_rgb(0xfa, 0xcb, 0x8d),
    accent_500: egui::Color32::from_rgb(0xc2, 0x8d, 0x41),
    accent_700: egui::Color32::from_rgb(0x7d, 0x54, 0x11),
    accent_800: egui::Color32::from_rgb(0x5a, 0x3b, 0x0a),
    neutral_200: egui::Color32::from_rgb(0xea, 0xe7, 0xe7),
    neutral_600: egui::Color32::from_rgb(0x7d, 0x79, 0x79),
    divider: egui::Color32::from_rgba_premultiplied(0x20, 0x1f, 0x1d, 40),
    is_dark: false,
};

thread_local! {
    /// The active window's resolved palette for the current frame. Set by
    /// `apply()`; stays at `CLASSICAL` for any window that never calls it.
    static PALETTE: Cell<Palette> = Cell::new(CLASSICAL);
}

pub fn bg() -> egui::Color32 { PALETTE.with(|p| p.get().bg) }
pub fn surface() -> egui::Color32 { PALETTE.with(|p| p.get().surface) }
pub fn text() -> egui::Color32 { PALETTE.with(|p| p.get().text) }
pub fn accent() -> egui::Color32 { PALETTE.with(|p| p.get().accent) }
pub fn accent_100() -> egui::Color32 { PALETTE.with(|p| p.get().accent_100) }
#[allow(dead_code)] // kept for API symmetry with the rest of the accent ramp
pub fn accent_300() -> egui::Color32 { PALETTE.with(|p| p.get().accent_300) }
pub fn accent_500() -> egui::Color32 { PALETTE.with(|p| p.get().accent_500) }
pub fn accent_700() -> egui::Color32 { PALETTE.with(|p| p.get().accent_700) }
pub fn accent_800() -> egui::Color32 { PALETTE.with(|p| p.get().accent_800) }
pub fn neutral_200() -> egui::Color32 { PALETTE.with(|p| p.get().neutral_200) }
pub fn neutral_600() -> egui::Color32 { PALETTE.with(|p| p.get().neutral_600) }
pub fn divider() -> egui::Color32 { PALETTE.with(|p| p.get().divider) }
pub fn is_dark() -> bool { PALETTE.with(|p| p.get().is_dark) }

/// Perceptual luminance in 0.0..1.0, used to decide whether a theme's base
/// colors read as a dark or light scheme (drives which direction tints get
/// generated in).
fn luminance(c: egui::Color32) -> f32 {
    (0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32) / 255.0
}

fn lerp_rgb(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t).round() as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t).round() as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t).round() as u8,
    )
}

fn lighten(c: egui::Color32, t: f32) -> egui::Color32 {
    lerp_rgb(c, egui::Color32::WHITE, t)
}

fn darken(c: egui::Color32, t: f32) -> egui::Color32 {
    lerp_rgb(c, egui::Color32::BLACK, t)
}

/// Derives a full `Palette` from a `Theme`'s 4 base colors
/// (`theme::palette`). The direction every tint/shade is generated in
/// depends on whether the theme reads as dark or light — mixing toward
/// white for a light theme's highlight backgrounds would make no sense for
/// a dark one (and vice versa for the "readable accent text" shades), so
/// this isn't the same fixed offsets reused across themes; each theme gets
/// tints that are actually legible and saturated against its own base.
pub fn palette_for(theme: Theme) -> Palette {
    let (bg, surface, accent, text) = crate::theme::palette(theme);
    let dark = luminance(bg) < 0.5;

    let (accent_100, accent_300, accent_700, accent_800, neutral_200) = if dark {
        (
            lerp_rgb(surface, accent, 0.22),
            lerp_rgb(surface, accent, 0.45),
            lighten(accent, 0.22),
            lighten(accent, 0.40),
            lerp_rgb(surface, text, 0.12),
        )
    } else {
        (
            lerp_rgb(egui::Color32::WHITE, accent, 0.12),
            lerp_rgb(egui::Color32::WHITE, accent, 0.38),
            darken(accent, 0.28),
            darken(accent, 0.45),
            lerp_rgb(egui::Color32::WHITE, text, 0.10),
        )
    };
    let accent_500 = if dark { lighten(accent, 0.08) } else { darken(accent, 0.10) };
    let neutral_600 = lerp_rgb(bg, text, 0.55);
    let divider = egui::Color32::from_rgba_premultiplied(text.r(), text.g(), text.b(), 40);

    Palette {
        bg,
        surface,
        text,
        accent,
        accent_100,
        accent_300,
        accent_500,
        accent_700,
        accent_800,
        neutral_200,
        neutral_600,
        divider,
        is_dark: dark,
    }
}

pub const RADIUS_MD: f32 = 4.0;
/// Card/dialog-scale corner radius — used by `card()` below and by the
/// corner timer's own card.
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

/// Resolves `theme`'s palette, stashes it in the thread-local so every
/// widget function below picks it up, and applies it to the egui visuals —
/// call once per frame before drawing the settings window's panels.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    let palette = palette_for(theme);
    PALETTE.with(|p| p.set(palette));

    let mut visuals = if palette.is_dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    visuals.override_text_color = Some(palette.text);
    visuals.window_fill = palette.bg;
    visuals.panel_fill = palette.bg;
    visuals.widgets.noninteractive.bg_fill = palette.surface;
    visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    visuals.widgets.inactive.weak_bg_fill = palette.surface;
    visuals.widgets.hovered.bg_fill = palette.accent_100;
    visuals.widgets.active.bg_fill = palette.accent_300;
    visuals.selection.bg_fill = palette.accent_300;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, palette.accent_700);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, palette.divider);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, palette.divider);
    // `extreme_bg_color` (text fields, sliders' groove) defaults to a
    // near-black egui left over from `Visuals::dark()`/`light()`'s own
    // presets — visibly clashing with our own themed surfaces if left
    // as-is (a near-black input on a themed dark-blue card, for example).
    // Tie it to the same palette instead.
    visuals.extreme_bg_color = if palette.is_dark {
        darken(palette.surface, 0.18)
    } else {
        lighten(palette.surface, 0.6)
    };
    ctx.set_visuals(visuals);
}

/// A soft drop shadow, matching CSS `box-shadow`'s look via egui's actual
/// blurred-shadow tessellation (`epaint::Shadow`) rather than a flat stroke.
/// Slightly stronger in dark themes, where a black shadow otherwise barely
/// registers against an already-dark page background.
pub fn card_shadow() -> egui::epaint::Shadow {
    let alpha = if is_dark() { 60 } else { 28 };
    egui::epaint::Shadow {
        offset: egui::vec2(0.0, 3.0),
        blur: 18.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(alpha),
    }
}

/// A raised content card: rounded corners, a hairline border, and a soft
/// shadow lifting it off the page background — matches the design's `.card`
/// treatment. The fill is a step lighter than `surface()` in both directions
/// (further toward white for light themes, a modest lift for dark ones,
/// mirroring how Material-style dark UIs raise elevated surfaces rather than
/// lightening all the way to white), so "elevated" reads correctly whichever
/// theme is active instead of hardcoding white.
pub fn card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let fill = if is_dark() {
        lighten(surface(), 0.12)
    } else {
        lighten(surface(), 0.9)
    };
    egui::Frame::none()
        .fill(fill)
        .rounding(RADIUS_LG)
        .stroke(egui::Stroke::new(1.0_f32, divider()))
        .shadow(card_shadow())
        .inner_margin(egui::Margin::symmetric(28.0, 24.0))
        .show(ui, add_contents)
        .inner
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
        // On-hover, nudge toward the "on" accent so the control visibly
        // previews what a click would do, same idea as a CSS `:hover` rule.
        let track_color = match (on, response.hovered()) {
            (true, _) => accent(),
            (false, true) => accent_100(),
            (false, false) => egui::Color32::TRANSPARENT,
        };
        let track_stroke = if on || response.hovered() { accent_700() } else { divider() };
        painter.rect_filled(rect, 999.0, track_color);
        painter.rect_stroke(rect, 999.0, egui::Stroke::new(1.0_f32, track_stroke));
        let knob_x = if on {
            rect.right() - KNOB_R - KNOB_GAP
        } else {
            rect.left() + KNOB_R + KNOB_GAP
        };
        let knob_center = egui::pos2(knob_x, rect.center().y);
        let knob_color = if on { egui::Color32::WHITE } else { neutral_600() };
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
        text(),
    );
    let padding = egui::vec2(12.0, 5.0);
    let size = galley.size() + padding * 2.0;
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let (fill, stroke, text_color) = if active {
            (accent_100(), accent(), accent_700())
        } else if response.hovered() {
            (neutral_200(), divider(), text())
        } else {
            (egui::Color32::TRANSPARENT, divider(), text())
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
            .stroke(egui::Stroke::new(1.0_f32, divider()))
            .rounding(RADIUS_MD)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, label) in options.iter().enumerate() {
                        let selected = i == current;
                        let (fill, text_color) = if selected {
                            (accent_100(), accent_700())
                        } else {
                            (egui::Color32::TRANSPARENT, text())
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
                                egui::Stroke::new(1.0_f32, divider()),
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

    painter.circle_stroke(center, radius, egui::Stroke::new(stroke_w, divider()));

    let start_deg = -90.0_f32;
    let end_deg = start_deg + fraction.clamp(0.0, 0.999) * 360.0;
    let steps = 48;
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = start_deg + (end_deg - start_deg) * (i as f32 / steps as f32);
        let rad = t.to_radians();
        points.push(center + egui::vec2(radius * rad.cos(), radius * rad.sin()));
    }
    painter.add(egui::Shape::line(points, egui::Stroke::new(stroke_w, accent())));

    painter.text(
        center + egui::vec2(0.0, -diameter * 0.06),
        egui::Align2::CENTER_CENTER,
        big_text,
        heading_font(diameter * 0.18),
        text(),
    );
    painter.text(
        center + egui::vec2(0.0, diameter * 0.11),
        egui::Align2::CENTER_CENTER,
        small_text,
        egui::FontId::proportional(diameter * 0.08),
        neutral_600(),
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
        let color = lerp_rgb(neutral_200(), accent(), *frac);

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

    painter.circle_filled(center, r_inner - 2.0, bg());
    painter.text(
        center + egui::vec2(0.0, -6.0),
        egui::Align2::CENTER_CENTER,
        "24h",
        egui::FontId::proportional(11.0),
        text(),
    );
    painter.text(
        center + egui::vec2(0.0, 7.0),
        egui::Align2::CENTER_CENTER,
        "activity",
        egui::FontId::proportional(9.0),
        neutral_600(),
    );
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

    painter.circle_stroke(center, radius, egui::Stroke::new(diameter * 0.06, divider()));

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
    hand(start_frac_of_12h, accent(), radius * 0.78);
    hand(end_frac_of_12h, accent_800(), radius * 0.78);
    painter.circle_filled(center, 3.0, text());

    for (label, frac) in [("12", 0.0_f32), ("3", 0.25), ("6", 0.5), ("9", 0.75)] {
        let deg: f32 = frac * 360.0 - 90.0;
        let rad = deg.to_radians();
        let pos = center + egui::vec2((radius + 12.0) * rad.cos(), (radius + 12.0) * rad.sin());
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(9.0),
            neutral_600(),
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
