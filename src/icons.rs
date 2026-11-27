//! Small vector icons drawn with the painter (no font or image dependencies).
use eframe::egui::{self, pos2, vec2, Color32, Painter, Pos2, Rect, Stroke, StrokeKind};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Overview,
    Memory,
    Cpu,
    Network,
    Battery,
    Health,
    SmartCare,
    Storage,
    Leftovers,
    Uninstaller,
    Startup,
    Assistant,
    AiSettings,
    Laptop,
    Reclaim,
}

/// A coloured rounded square with a white glyph, used for sidebar rows and card headers.
pub fn tile(p: &Painter, icon: Icon, center: Pos2, size: f32, color: Color32) {
    let r = Rect::from_center_size(center, vec2(size, size));
    p.rect_filled(r, size * 0.28, color);
    p.rect_filled(Rect::from_min_size(r.min + vec2(1.5, 1.5), vec2(size - 3.0, size * 0.28)), size * 0.2, crate::theme::veil(34));
    draw(p, icon, center, Color32::WHITE);
}

/// Small vector icons drawn with the painter (no font or image dependencies).
pub fn draw(p: &Painter, icon: Icon, c: Pos2, col: Color32) {
    let st = Stroke::new(1.6_f32, col);
    let line = |a: Pos2, b: Pos2| p.line_segment([c + a.to_vec2(), c + b.to_vec2()], st);
    let rect = |w: f32, h: f32, r: f32| p.rect_stroke(Rect::from_center_size(c, vec2(w, h)), r, st, StrokeKind::Middle);
    match icon {
        Icon::Overview => {
            for (dx, dy) in [(-4.5, -4.5), (4.5, -4.5), (-4.5, 4.5), (4.5, 4.5)] {
                p.rect_stroke(Rect::from_center_size(c + vec2(dx, dy), vec2(6.0, 6.0)), 1.5, st, StrokeKind::Middle);
            }
        }
        Icon::Memory => {
            rect(14.0, 8.0, 2.0);
            for x in [-4.0, 0.0, 4.0] {
                line(pos2(x, -4.0), pos2(x, -7.0));
                line(pos2(x, 4.0), pos2(x, 7.0));
            }
        }
        Icon::Cpu => {
            rect(11.0, 11.0, 2.5);
            p.rect_filled(Rect::from_center_size(c, vec2(4.0, 4.0)), 1.0, col);
            for v in [-3.0, 3.0] {
                line(pos2(v, -5.5), pos2(v, -8.0));
                line(pos2(v, 5.5), pos2(v, 8.0));
                line(pos2(-5.5, v), pos2(-8.0, v));
                line(pos2(5.5, v), pos2(8.0, v));
            }
        }
        Icon::Network => {
            line(pos2(-4.0, 7.0), pos2(-4.0, -6.0));
            line(pos2(-4.0, -7.0), pos2(-7.0, -3.5));
            line(pos2(-4.0, -7.0), pos2(-1.0, -3.5));
            line(pos2(4.0, -7.0), pos2(4.0, 6.0));
            line(pos2(4.0, 7.0), pos2(1.0, 3.5));
            line(pos2(4.0, 7.0), pos2(7.0, 3.5));
        }
        Icon::Battery => {
            p.rect_stroke(Rect::from_center_size(c + vec2(-1.0, 0.0), vec2(14.0, 8.5)), 2.0, st, StrokeKind::Middle);
            p.rect_filled(Rect::from_center_size(c + vec2(7.5, 0.0), vec2(2.0, 4.0)), 1.0, col);
            p.rect_filled(Rect::from_center_size(c + vec2(-3.0, 0.0), vec2(7.0, 4.5)), 1.0, col);
        }
        Icon::Health => {
            let pts = [(-8.0, 0.5), (-4.0, 0.5), (-2.0, -6.0), (1.0, 6.0), (3.0, 0.5), (8.0, 0.5)];
            for w in pts.windows(2) {
                line(pos2(w[0].0, w[0].1), pos2(w[1].0, w[1].1));
            }
        }
        Icon::SmartCare => {
            let pts: Vec<Pos2> = [(0.0, -8.0), (2.0, -2.0), (8.0, 0.0), (2.0, 2.0), (0.0, 8.0), (-2.0, 2.0), (-8.0, 0.0), (-2.0, -2.0)]
                .iter()
                .map(|(x, y)| c + vec2(*x, *y))
                .collect();
            p.add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
        }
        Icon::Storage => {
            p.circle_stroke(c, 7.5, st);
            p.circle_stroke(c, 2.5, st);
            line(pos2(2.5, -2.5), pos2(6.0, -6.0));
        }
        Icon::Leftovers => {
            p.rect_stroke(Rect::from_center_size(c + vec2(0.0, 1.5), vec2(11.0, 11.0)), 2.0, st, StrokeKind::Middle);
            line(pos2(-7.0, -5.0), pos2(7.0, -5.0));
            line(pos2(-2.5, -5.0), pos2(-2.5, -7.5));
            line(pos2(2.5, -5.0), pos2(2.5, -7.5));
            line(pos2(-2.5, -1.0), pos2(-2.5, 5.0));
            line(pos2(2.5, -1.0), pos2(2.5, 5.0));
        }
        Icon::Uninstaller => {
            rect(13.0, 13.0, 3.0);
            line(pos2(-3.5, -3.5), pos2(3.5, 3.5));
            line(pos2(3.5, -3.5), pos2(-3.5, 3.5));
        }
        Icon::Startup => {
            let pts: Vec<Pos2> = (0..=36)
                .map(|i| {
                    let a = (-60.0_f32 + 300.0 * i as f32 / 36.0 - 90.0).to_radians();
                    c + vec2(a.cos(), a.sin()) * 7.0
                })
                .collect();
            p.add(egui::Shape::line(pts, st));
            line(pos2(0.0, -8.5), pos2(0.0, -1.0));
        }
        Icon::Assistant => {
            p.rect_stroke(Rect::from_center_size(c + vec2(0.0, -1.0), vec2(15.0, 10.5)), 3.0, st, StrokeKind::Middle);
            line(pos2(-3.0, 4.5), pos2(-5.5, 8.0));
            line(pos2(-3.0, 4.5), pos2(1.0, 4.5));
        }
        Icon::Laptop => {
            p.rect_stroke(Rect::from_center_size(c + vec2(0.0, -2.0), vec2(16.0, 11.0)), 2.5, st, StrokeKind::Middle);
            line(pos2(-10.0, 6.5), pos2(10.0, 6.5));
        }
        Icon::Reclaim => {
            line(pos2(0.0, -7.0), pos2(0.0, 5.0));
            line(pos2(0.0, 6.0), pos2(-4.0, 2.0));
            line(pos2(0.0, 6.0), pos2(4.0, 2.0));
            line(pos2(-7.0, 8.0), pos2(7.0, 8.0));
        }
        Icon::AiSettings => {
            for (y, x) in [(-5.0, -3.0), (0.0, 3.0), (5.0, -1.0)] {
                line(pos2(-8.0, y), pos2(8.0, y));
                p.circle_filled(c + vec2(x, y), 2.4, col);
            }
        }
    }
}

