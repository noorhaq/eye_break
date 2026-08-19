use eframe::egui;
use serde::{Deserialize, Serialize};

/// Visual theme applied to eye-break's egui windows (overlay, corner timer).
/// Self-contained: `apply` is the only entry point other modules need.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    Solarized,
    HighContrast,
    Nord,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

impl Theme {
    /// All available themes, in the order they should be presented in a UI.
    pub fn all() -> &'static [Theme] {
        &[
            Theme::Dark,
            Theme::Light,
            Theme::Solarized,
            Theme::HighContrast,
            Theme::Nord,
        ]
    }

    /// Human-readable label for menus/pickers.
    pub fn label(&self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Solarized => "Solarized",
            Theme::HighContrast => "High Contrast",
            Theme::Nord => "Nord",
        }
    }
}

/// Apply the given theme's palette to an egui context's visuals/style.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    let mut visuals = match theme {
        Theme::Dark => egui::Visuals::dark(),
        Theme::Light => egui::Visuals::light(),
        Theme::Solarized => solarized_visuals(),
        Theme::HighContrast => high_contrast_visuals(),
        Theme::Nord => nord_visuals(),
    };

    let (bg, panel_bg, accent, text) = palette(theme);
    visuals.override_text_color = Some(text);
    visuals.widgets.noninteractive.bg_fill = panel_bg;
    visuals.widgets.inactive.bg_fill = panel_bg;
    visuals.widgets.active.bg_fill = accent;
    visuals.widgets.hovered.bg_fill = accent;
    visuals.selection.bg_fill = accent;
    visuals.window_fill = bg;
    visuals.panel_fill = bg;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.visuals = ctx.style().visuals.clone();
    ctx.set_style(style);
}

/// (window background, panel background, accent, text) for a theme.
fn palette(theme: Theme) -> (egui::Color32, egui::Color32, egui::Color32, egui::Color32) {
    match theme {
        Theme::Dark => (
            egui::Color32::from_rgb(24, 24, 27),
            egui::Color32::from_rgb(32, 32, 36),
            egui::Color32::from_rgb(90, 140, 255),
            egui::Color32::from_rgb(230, 230, 230),
        ),
        Theme::Light => (
            egui::Color32::from_rgb(248, 248, 248),
            egui::Color32::from_rgb(255, 255, 255),
            egui::Color32::from_rgb(50, 110, 220),
            egui::Color32::from_rgb(30, 30, 30),
        ),
        Theme::Solarized => (
            egui::Color32::from_rgb(0, 43, 54),
            egui::Color32::from_rgb(7, 54, 66),
            egui::Color32::from_rgb(181, 137, 0),
            egui::Color32::from_rgb(131, 148, 150),
        ),
        Theme::HighContrast => (
            egui::Color32::from_rgb(0, 0, 0),
            egui::Color32::from_rgb(0, 0, 0),
            egui::Color32::from_rgb(255, 255, 0),
            egui::Color32::from_rgb(255, 255, 255),
        ),
        Theme::Nord => (
            egui::Color32::from_rgb(46, 52, 64),
            egui::Color32::from_rgb(59, 66, 82),
            egui::Color32::from_rgb(136, 192, 208),
            egui::Color32::from_rgb(216, 222, 233),
        ),
    }
}

fn solarized_visuals() -> egui::Visuals {
    egui::Visuals::dark()
}

fn high_contrast_visuals() -> egui::Visuals {
    egui::Visuals::dark()
}

fn nord_visuals() -> egui::Visuals {
    egui::Visuals::dark()
}
