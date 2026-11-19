//! Motion tokens (DESIGN 2.6) and reduce-motion. Animations only request repaints while running.

use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};

static REDUCED: AtomicBool = AtomicBool::new(false);

/// Windows "Show animations" off → all durations 0 (progress smoothing stays).
pub fn set_reduced(r: bool) {
    REDUCED.store(r, Ordering::Relaxed);
}

pub fn reduced() -> bool {
    REDUCED.load(Ordering::Relaxed)
}

/// Token durations in seconds; 0 when motion is reduced.
pub fn dur(token: f32) -> f32 {
    if reduced() { 0.0 } else { token }
}

pub const PRESS: f32 = 0.06;
pub const HOVER: f32 = 0.10;
pub const TAB: f32 = 0.18;
pub const DIALOG_IN: f32 = 0.18;
pub const DIALOG_OUT: f32 = 0.12;
pub const ITEM_IN: f32 = 0.20;
pub const REORDER: f32 = 0.18;
pub const THEME: f32 = 0.30;
pub const PROGRESS_TAU: f32 = 0.25;

/// Eased 0→1 since `start` (seconds, egui input time); `None` once finished.
pub fn progress(
    ctx: &egui::Context,
    start: f64,
    duration: f32,
    ease: fn(f32) -> f32,
) -> Option<f32> {
    let d = dur(duration);
    if d <= 0.0 {
        return None;
    }
    let t = ((ctx.input(|i| i.time) - start) as f32 / d).clamp(0.0, 1.0);
    if t >= 1.0 {
        None
    } else {
        ctx.request_repaint();
        Some(ease(t))
    }
}

/// `motion.progress`: exponential approach, keyed by `id`; returns the shown value.
pub fn smooth(ctx: &egui::Context, id: egui::Id, target: f32) -> f32 {
    let dt = ctx.input(|i| i.stable_dt).min(0.1);
    let shown = ctx.data_mut(|d| {
        let v = d.get_temp_mut_or(id, target);
        *v += (target - *v) * (1.0 - (-dt / PROGRESS_TAU).exp());
        if (target - *v).abs() < 0.001 {
            *v = target;
        }
        *v
    });
    if shown != target {
        ctx.request_repaint();
    }
    shown
}
