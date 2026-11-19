//! Splash (DESIGN 4.1): procedural logo scene, startup steps on a worker thread, root-viewport morph.

use super::{anim, fonts, theme};
use crate::{config::Config, i18n::I18n, net};
use eframe::egui::{self, Align2, Color32, Mesh, Pos2, Rect, Stroke, ViewportCommand, pos2, vec2};
use std::{
    net::{Ipv4Addr, TcpListener},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

pub const SIZE: [f32; 2] = [480.0, 300.0];
const MIN_SECS: f64 = 1.2;
const MAX_SECS: f64 = 5.0;
const STEPS: usize = 4;

pub struct Splash {
    start: f64,
    done_steps: usize,
    step_key: &'static str,
    rx: Receiver<(usize, &'static str)>,
    skip: bool,
    closing: Option<f64>,
    /// Warning from a step (shown later as a log entry / banner).
    pub warning: Option<String>,
}

/// Step texts are i18n keys; the worker only reports which step finished.
fn spawn_steps(cfg: Config, queue_loaded: Receiver<()>) -> Receiver<(usize, &'static str)> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send((1, "splash.step.queue")); // 1: settings were loaded before the window opened
        let _ = queue_loaded.recv_timeout(Duration::from_secs(3));
        let _ = tx.send((2, "splash.step.server"));
        let pc = cfg.console_ip().map(|ip| {
            cfg.pc_ip_override()
                .map_or_else(|| net::pc_ip_for(ip, cfg.console_port), Ok)
        });
        let _ = tx.send((3, "splash.step.port"));
        let ip = match pc {
            Some(Ok(ip)) => ip,
            _ => Ipv4Addr::UNSPECIFIED,
        };
        // the real bind happens when the queue starts (deferred bind, DESIGN 6.6); here only "is it free"
        let free = TcpListener::bind((ip, cfg.server_port)).is_ok();
        let _ = tx.send((
            4,
            if free {
                "splash.step.done"
            } else {
                "splash.step.port_busy"
            },
        ));
    });
    rx
}

impl Splash {
    pub fn new(now: f64, cfg: &Config, queue_loaded: Receiver<()>) -> Self {
        Self {
            start: now,
            done_steps: 0,
            step_key: "splash.step.settings",
            rx: spawn_steps(cfg.clone(), queue_loaded),
            skip: false,
            closing: None,
            warning: None,
        }
    }

    /// Draws the splash. Returns true once it has finished (the caller morphs the window).
    pub fn ui(&mut self, ui: &mut egui::Ui, t: &I18n) -> bool {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        while let Ok((n, key)) = self.rx.try_recv() {
            self.done_steps = n;
            self.step_key = key;
            if key == "splash.step.port_busy" {
                self.warning = Some(t.t(key));
            }
        }
        if ctx.input(|i| {
            i.pointer.any_click()
                || i.key_pressed(egui::Key::Escape)
                || i.key_pressed(egui::Key::Enter)
                || i.key_pressed(egui::Key::Space)
        }) {
            self.skip = true;
        }
        let el = now - self.start;
        let steps_done = self.done_steps >= STEPS;
        let min = if self.skip || anim::reduced() {
            0.0
        } else {
            MIN_SECS
        };
        if self.closing.is_none() && ((steps_done && el >= min) || el >= MAX_SECS) {
            self.closing = Some(now);
        }
        let fade_out = self
            .closing
            .map_or(1.0, |c| 1.0 - ((now - c) / 0.2).clamp(0.0, 1.0) as f32);
        if self.closing.is_some_and(|c| now - c >= 0.2)
            || (self.closing.is_some() && anim::reduced())
        {
            return true;
        }
        let r = ui.max_rect();
        let t_ms = if self.skip || anim::reduced() {
            10_000.0
        } else {
            (el * 1000.0) as f32
        };
        let alpha = (t_ms / 150.0).clamp(0.0, 1.0) * fade_out;
        draw_scene(ui.painter(), r, t_ms, alpha);
        let txt = |c: Color32| c.gamma_multiply(alpha);
        let p = theme::DARK;
        ui.painter().text(
            pos2(r.center().x, r.top() + 218.0),
            Align2::CENTER_CENTER,
            "PS Package Pusher",
            fonts::display(fonts::size::DISPLAY),
            txt(p.text_primary),
        );
        ui.painter().text(
            pos2(r.center().x, r.top() + 244.0),
            Align2::CENTER_CENTER,
            format!("v{}", env!("CARGO_PKG_VERSION")),
            fonts::regular(fonts::size::CAPTION),
            txt(p.text_secondary),
        );
        ui.painter().text(
            pos2(r.left() + 20.0, r.bottom() - 26.0),
            Align2::LEFT_CENTER,
            t.t(self.step_key),
            fonts::regular(fonts::size::CAPTION),
            txt(p.text_secondary),
        );
        let line = Rect::from_min_size(
            pos2(r.left() + 20.0, r.bottom() - 14.0),
            vec2(r.width() - 40.0, 2.0),
        );
        ui.painter().rect_filled(line, 1, txt(p.track));
        let shown = anim::smooth(
            &ctx,
            egui::Id::unique("splash-progress"),
            self.done_steps as f32 / STEPS as f32,
        );
        ui.painter().rect_filled(
            Rect::from_min_size(line.min, vec2(line.width() * shown, 2.0)),
            1,
            txt(p.accent),
        );
        if t_ms < 1200.0 || self.closing.is_some() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(100)); // waiting for steps only
        }
        false
    }
}

/// Root viewport becomes the main window (DESIGN 4.1 morph; validated in spike S-1).
pub fn morph(ctx: &egui::Context, cfg: &Config) {
    let monitor = ctx
        .input(|i| i.viewport().monitor_size)
        .unwrap_or(vec2(1920.0, 1080.0));
    let (size, pos) = match cfg.window {
        Some(w) => (vec2(w.w.max(880.0), w.h.max(560.0)), Pos2::new(w.x, w.y)),
        None => {
            let s = vec2(1100.0, 720.0);
            (
                s,
                Pos2::new(
                    ((monitor.x - s.x) / 2.0).max(0.0),
                    ((monitor.y - s.y) / 2.0).max(0.0),
                ),
            )
        }
    };
    for cmd in [
        ViewportCommand::Transparent(false),
        ViewportCommand::Decorations(true),
        ViewportCommand::Resizable(true),
        ViewportCommand::MinInnerSize(vec2(880.0, 560.0)),
        ViewportCommand::InnerSize(size),
        ViewportCommand::OuterPosition(pos),
    ] {
        ctx.send_viewport_cmd(cmd);
    }
    if cfg.window.is_some_and(|w| w.maximized) {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(true));
    }
}

fn gamma(c: Color32, a: f32) -> Color32 {
    c.gamma_multiply(a.clamp(0.0, 1.0))
}

fn k(t: f32, from: f32, to: f32, ease: fn(f32) -> f32) -> f32 {
    ease(((t - from) / (to - from)).clamp(0.0, 1.0))
}

/// Concept 4 scene drawn with the painter (sharp at any DPI, animatable). `t` in ms.
fn draw_scene(painter: &egui::Painter, r: Rect, t: f32, alpha: f32) {
    use egui::emath::easing::{back_out, cubic_in_out, cubic_out, linear};
    let [y0, y1, y2] = theme::SUNSET;
    painter.rect_filled(r, 16, gamma(theme::DARK.bg_base, alpha));
    let cx = r.center().x;
    let horizon = r.top() + 150.0;
    // sun: rises 40 px, stripes cut its lower half
    let ks = k(t, 100.0, 700.0, cubic_out);
    let sun_c = pos2(cx, r.top() + 92.0 + 40.0 * (1.0 - ks));
    let rad = 66.0;
    let mut mesh = Mesh::default();
    let rows = 40;
    for i in 0..rows {
        let (fa, fb) = (i as f32 / rows as f32, (i + 1) as f32 / rows as f32);
        let (ya, yb) = (
            sun_c.y - rad + 2.0 * rad * fa,
            sun_c.y - rad + 2.0 * rad * fb,
        );
        if yb > horizon {
            break;
        }
        let half = |y: f32| (rad * rad - (y - sun_c.y).powi(2)).max(0.0).sqrt();
        let col = |f: f32| {
            let c = if f < 0.45 {
                y0.lerp_to_gamma(y1, f / 0.45)
            } else {
                y1.lerp_to_gamma(y2, (f - 0.45) / 0.55)
            };
            gamma(c, alpha * ks.max(0.001))
        };
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(pos2(cx - half(ya), ya), col(fa));
        mesh.colored_vertex(pos2(cx + half(ya), ya), col(fa));
        mesh.colored_vertex(pos2(cx + half(yb), yb), col(fb));
        mesh.colored_vertex(pos2(cx - half(yb), yb), col(fb));
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    painter.add(mesh);
    for (dy, h) in [(14.0, 3.0), (30.0, 5.0), (46.0, 7.0)] {
        let y = sun_c.y + dy;
        if y < horizon {
            painter.rect_filled(
                Rect::from_min_size(pos2(cx - rad, y), vec2(2.0 * rad, h)),
                0,
                gamma(theme::DARK.bg_base, alpha),
            );
        }
    }
    // floor + grid lines lighting up from the horizon
    let floor = Rect::from_min_max(pos2(r.left(), horizon), pos2(r.right(), r.top() + 196.0));
    painter.rect_filled(floor, 0, gamma(Color32::from_rgb(0x1A, 0x06, 0x38), alpha));
    for (i, y) in [6.0, 14.0, 26.0, 42.0].iter().enumerate() {
        let a = k(
            t,
            300.0 + 40.0 * i as f32,
            600.0 + 40.0 * i as f32,
            cubic_out,
        );
        painter.line_segment(
            [pos2(r.left(), horizon + y), pos2(r.right(), horizon + y)],
            Stroke::new(1.2, gamma(theme::DARK.accent2, alpha * a * 0.85)),
        );
    }
    let ag = k(t, 300.0, 900.0, cubic_out);
    for i in -6..=6 {
        let x = cx + i as f32 * 70.0;
        painter.line_segment(
            [
                pos2(cx, horizon),
                pos2(x * 1.0 + (x - cx) * 1.5, floor.bottom()),
            ],
            Stroke::new(1.2, gamma(theme::DARK.accent2, alpha * ag * 0.85)),
        );
    }
    let flash = k(t, 300.0, 360.0, linear);
    painter.line_segment(
        [pos2(r.left(), horizon), pos2(r.right(), horizon)],
        Stroke::new(
            1.5 + 2.0 * (1.0 - k(t, 360.0, 900.0, linear)) * flash,
            gamma(theme::DARK.accent, alpha * flash),
        ),
    );
    // box (back_out scale) and the arrow flying through it
    let kb = k(t, 500.0, 800.0, back_out);
    let s = 0.9 + 0.1 * kb;
    let bc = pos2(cx, horizon + 6.0);
    let pt = |x: f32, y: f32| pos2(bc.x + x * s, bc.y + y * s);
    let cyan = gamma(theme::DARK.accent, alpha * kb.min(1.0));
    let face = |pts: [Pos2; 4], fill: Color32| {
        painter.add(egui::Shape::convex_polygon(
            pts.to_vec(),
            gamma(fill, alpha * kb.min(1.0)),
            Stroke::new(2.0, cyan),
        ));
    };
    let ka = k(t, 750.0, 1150.0, cubic_in_out);
    let ax = r.left() - 40.0 + (cx + 70.0 - r.left() + 40.0) * ka;
    let ay = bc.y + 4.0;
    if ka > 0.0 {
        let speed = 1.0 - k(t, 1150.0, 1350.0, linear);
        for (dy, len) in [(-8.0, 46.0), (10.0, 60.0)] {
            painter.line_segment(
                [pos2(ax - 90.0 - len, ay + dy), pos2(ax - 90.0, ay + dy)],
                Stroke::new(3.0, gamma(theme::DARK.accent2, alpha * speed.max(0.0))),
            );
        }
        painter.rect_filled(
            Rect::from_min_max(
                pos2((ax - 90.0).max(r.left()), ay - 4.0),
                pos2(ax.min(cx - 30.0), ay + 4.0),
            ),
            2,
            gamma(theme::DARK.accent, alpha),
        );
    }
    face(
        [
            pt(-30.0, -10.0),
            pt(0.0, 5.0),
            pt(0.0, 44.0),
            pt(-30.0, 29.0),
        ],
        Color32::from_rgb(0x2A, 0x0A, 0x5E),
    );
    face(
        [pt(0.0, 5.0), pt(30.0, -10.0), pt(30.0, 29.0), pt(0.0, 44.0)],
        Color32::from_rgb(0x4A, 0x15, 0x97),
    );
    face(
        [
            pt(0.0, -25.0),
            pt(30.0, -10.0),
            pt(0.0, 5.0),
            pt(-30.0, -10.0),
        ],
        Color32::from_rgb(0x7B, 0x2F, 0xF7),
    );
    if ka > 0.0 && ax > cx + 30.0 {
        let head = [
            pos2(cx + 30.0, ay - 4.0),
            pos2(ax - 16.0, ay - 4.0),
            pos2(ax - 16.0, ay - 12.0),
            pos2(ax, ay),
            pos2(ax - 16.0, ay + 12.0),
            pos2(ax - 16.0, ay + 4.0),
            pos2(cx + 30.0, ay + 4.0),
        ];
        painter.add(egui::Shape::closed_line(
            head.to_vec(),
            Stroke::new(1.0, gamma(theme::DARK.accent, alpha)),
        ));
        painter.add(egui::Shape::convex_polygon(
            vec![head[1], head[2], head[3], head[4], head[5]],
            gamma(theme::DARK.accent, alpha),
            Stroke::NONE,
        ));
        painter.rect_filled(
            Rect::from_min_max(head[0], head[5]),
            0,
            gamma(theme::DARK.accent, alpha),
        );
    }
}
