//! Shared widgets: buttons, badge, status chip, progress bar, banner, focus ring (DESIGN 2.2, 2.4).

use super::{
    anim,
    fonts::{self, icon, size},
    theme::Palette,
};
use eframe::egui::{
    self, Color32, CornerRadius, Mesh, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2,
    vec2,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Danger,
    /// Text-only, for toolbars.
    Ghost,
}

/// Focus ring: 2 px + 2 px gap; glow only in dark (DESIGN 2.2 glow rules).
pub fn focus_ring(ui: &Ui, rect: Rect, p: &Palette, radius: f32) {
    let r = rect.expand(4.0);
    if p.dark {
        for (grow, alpha) in [(4.0, 26u8), (2.0, 64)] {
            let c = Color32::from_rgba_unmultiplied(p.focus.r(), p.focus.g(), p.focus.b(), alpha);
            ui.painter()
                .rect_filled(r.expand(grow), radius + 4.0 + grow, c);
        }
    }
    ui.painter().rect_stroke(
        r,
        radius + 4.0,
        Stroke::new(2.0, p.focus),
        StrokeKind::Inside,
    );
}

fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    a.lerp_to_gamma(b, t)
}

pub fn button(ui: &mut Ui, p: &Palette, kind: Kind, icon: Option<&str>, label: &str) -> Response {
    button_enabled(ui, p, kind, icon, label, true)
}

pub fn button_enabled(
    ui: &mut Ui,
    p: &Palette,
    kind: Kind,
    icon_s: Option<&str>,
    label: &str,
    enabled: bool,
) -> Response {
    let mk = |c: Color32| fonts::job(icon_s.unwrap_or(""), label, fonts::semibold(size::BODY), c);
    let galley = ui.painter().layout_job(mk(Color32::WHITE));
    let pad = vec2(if kind == Kind::Ghost { 8.0 } else { 14.0 }, 0.0);
    let desired = vec2((galley.size().x + pad.x * 2.0).max(32.0), 32.0);
    let (rect, resp) = ui.allocate_exact_size(
        desired,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hover = ui.ctx().animate_bool_with_time(
        resp.id.with("h"),
        enabled && resp.hovered(),
        anim::dur(anim::HOVER),
    );
    let press = ui.ctx().animate_bool_with_time(
        resp.id.with("p"),
        enabled && resp.is_pointer_button_down_on(),
        anim::dur(anim::PRESS),
    );
    let (fill, fg, stroke) = match kind {
        Kind::Primary => (p.accent, p.accent_on, None),
        Kind::Danger => (p.danger, p.danger_on, None),
        Kind::Secondary => (p.bg_surface2, p.text_primary, Some(p.border_strong)),
        Kind::Ghost => (Color32::TRANSPARENT, p.text_primary, None),
    };
    let hover_fill = match kind {
        Kind::Primary | Kind::Danger => blend(fill, p.text_primary, 0.12),
        _ => p.bg_hover,
    };
    let mut bg = blend(fill, hover_fill, hover);
    bg = blend(bg, p.bg_selected, press * 0.5);
    let (bg, fg) = if enabled {
        (bg, fg)
    } else {
        (
            if kind == Kind::Ghost {
                bg
            } else {
                p.bg_surface2
            },
            p.text_disabled,
        )
    };
    let r = CornerRadius::same(8);
    ui.painter().rect_filled(rect, r, bg);
    if let Some(s) = stroke {
        ui.painter().rect_stroke(
            rect,
            r,
            Stroke::new(1.0, if enabled { s } else { p.border_subtle }),
            StrokeKind::Inside,
        );
    }
    let g = ui.painter().layout_job(mk(fg));
    ui.painter().galley(rect.center() - g.size() / 2.0, g, fg);
    if resp.has_focus() {
        focus_ring(ui, rect, p, 8.0);
    }
    resp.on_hover_cursor(if enabled {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Default
    })
}

/// Pill badge, uppercase (DESIGN 2.3 `type.badge`).
pub fn badge(ui: &mut Ui, colors: (Color32, Color32), label: &str) -> Response {
    let text = label.to_uppercase();
    let galley = ui
        .painter()
        .layout_no_wrap(text, fonts::semibold(size::BADGE), colors.1);
    let size = galley.size() + vec2(14.0, 6.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_filled(rect, rect.height() / 2.0, colors.0);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, colors.1);
    resp
}

/// Status = icon + text in one colour, never colour alone (DESIGN 2.2).
pub fn status(ui: &mut Ui, color: Color32, icon_s: &str, text: &str) -> Response {
    ui.label(fonts::job(icon_s, text, fonts::regular(size::BODY), color))
}

fn gradient_rect(ui: &Ui, rect: Rect, a: Color32, b: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(rect.left_top(), a);
    m.colored_vertex(rect.right_top(), b);
    m.colored_vertex(rect.right_bottom(), b);
    m.colored_vertex(rect.left_bottom(), a);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    ui.painter().add(m);
}

/// Progress bar 8 px. `fraction = None` → indeterminate segment (≤ 30 fps, DESIGN 2.6).
pub fn progress(
    ui: &mut Ui,
    p: &Palette,
    fraction: Option<f32>,
    width: f32,
    id: egui::Id,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 8.0), Sense::hover());
    let r = CornerRadius::same(4);
    ui.painter().rect_filled(rect, r, p.track);
    match fraction {
        Some(f) => {
            let shown = anim::smooth(ui.ctx(), id, f.clamp(0.0, 1.0));
            if shown > 0.0 {
                let fill = Rect::from_min_size(rect.min, vec2(rect.width() * shown, rect.height()));
                let end = blend(p.progress_a, p.progress_b, shown);
                if p.dark {
                    let g = Color32::from_rgba_unmultiplied(
                        p.progress_a.r(),
                        p.progress_a.g(),
                        p.progress_a.b(),
                        40,
                    );
                    ui.painter().rect_filled(fill.expand(2.0), 6.0, g);
                }
                gradient_rect(ui, fill, p.progress_a, end);
            }
        }
        None => {
            let t = ui.input(|i| i.time) as f32;
            let phase = if anim::reduced() {
                0.35
            } else {
                (t / 1.2).fract()
            };
            let seg_w = rect.width() * 0.3;
            let x = rect.left() - seg_w + (rect.width() + seg_w) * phase;
            let seg = Rect::from_min_max(
                pos2(x.max(rect.left()), rect.top()),
                pos2((x + seg_w).min(rect.right()), rect.bottom()),
            );
            if seg.width() > 0.0 {
                ui.painter().rect_filled(seg, r, p.accent);
            }
            if !anim::reduced() {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(33));
            }
        }
    }
    resp
}

#[derive(Clone, Copy)]
pub enum BannerKind {
    Info,
    Warning,
    Danger,
    Success,
}

/// Full-width banner with an optional action button; returns true when the action was clicked.
pub fn banner(
    ui: &mut Ui,
    p: &Palette,
    kind: BannerKind,
    title: &str,
    text: &str,
    action: Option<&str>,
) -> bool {
    let (bg, fg, ic) = match kind {
        BannerKind::Info => (p.banner_info, p.info, icon::INFO),
        BannerKind::Warning => (p.banner_warning, p.warning, icon::WARNING),
        BannerKind::Danger => (p.banner_danger, p.danger, icon::WARNING_OCTAGON),
        BannerKind::Success => (p.banner_success, p.success, icon::CHECK),
    };
    let mut clicked = false;
    egui::Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(16, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(fonts::icon_rt(ic, size::HEADING).color(fg));
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(title)
                            .color(fg)
                            .font(fonts::semibold(size::BODY)),
                    );
                    if !text.is_empty() {
                        ui.label(
                            egui::RichText::new(text)
                                .color(p.text_primary)
                                .font(fonts::regular(size::CAPTION)),
                        );
                    }
                });
                if let Some(a) = action {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        clicked = button(ui, p, Kind::Secondary, None, a).clicked();
                    });
                }
            });
        });
    clicked
}

/// 2 px sunset stripe at the top of the window (DESIGN 3, `decor.sunset`).
pub fn sunset_stripe(ui: &Ui, rect: Rect) {
    let mid = pos2(rect.center().x, rect.top());
    let [a, b, c] = super::theme::SUNSET;
    let left = Rect::from_min_max(rect.left_top(), Pos2::new(mid.x, rect.top() + 2.0));
    let right = Rect::from_min_max(
        Pos2::new(mid.x, rect.top()),
        Pos2::new(rect.right(), rect.top() + 2.0),
    );
    gradient_rect(ui, left, a, b);
    gradient_rect(ui, right, b, c);
}

/// Section card used by Settings/Log.
pub fn card<R>(ui: &mut Ui, p: &Palette, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(p.bg_surface)
        .stroke(Stroke::new(1.0, p.border_subtle))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Vertical spacing tokens (DESIGN 2.4).
pub fn space(ui: &mut Ui, n: usize) {
    ui.add_space([4.0, 8.0, 12.0, 16.0, 24.0, 32.0][n.clamp(1, 6) - 1]);
}
