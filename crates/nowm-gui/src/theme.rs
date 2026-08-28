//! Colours and spacing.
//!
//! egui's defaults are perfectly usable; what they lack is a palette that
//! matches the severity vocabulary of the report. Those colours are defined
//! once here so the badge, the table and the payload panel always agree.

use egui::{Color32, Context, Stroke, Visuals};
use nowm_core::{Severity, Verdict};

pub const ACCENT: Color32 = Color32::from_rgb(0x5B, 0xC8, 0xEB);
pub const CRITICAL: Color32 = Color32::from_rgb(0xD9, 0x7A, 0xF0);
pub const HIGH: Color32 = Color32::from_rgb(0xF2, 0x6D, 0x6D);
pub const MEDIUM: Color32 = Color32::from_rgb(0xE8, 0xB3, 0x39);
pub const LOW: Color32 = Color32::from_rgb(0x6F, 0xA8, 0xF5);
pub const INFO: Color32 = Color32::from_rgb(0x9A, 0xA3, 0xB2);
pub const OK: Color32 = Color32::from_rgb(0x5E, 0xC9, 0x8C);

pub fn severity_color(s: Severity) -> Color32 {
    match s {
        Severity::Critical => CRITICAL,
        Severity::High => HIGH,
        Severity::Medium => MEDIUM,
        Severity::Low => LOW,
        Severity::Info => INFO,
    }
}

pub fn verdict_color(v: Verdict) -> Color32 {
    match v {
        Verdict::HiddenContent => CRITICAL,
        Verdict::Suspicious => HIGH,
        Verdict::Cosmetic => MEDIUM,
        Verdict::Clean => OK,
    }
}

/// Apply the light or dark palette, plus the handful of tweaks that make the
/// window feel like an application rather than a debug overlay.
pub fn apply(ctx: &Context, dark: bool) {
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.45);
    visuals.hyperlink_color = ACCENT;
    if dark {
        visuals.panel_fill = Color32::from_rgb(0x14, 0x17, 0x1F);
        visuals.window_fill = Color32::from_rgb(0x14, 0x17, 0x1F);
        visuals.extreme_bg_color = Color32::from_rgb(0x0E, 0x10, 0x17);
        visuals.widgets.noninteractive.bg_stroke =
            Stroke::new(1.0, Color32::from_rgb(0x25, 0x2A, 0x36));
    }
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    ctx.set_style(style);
}
