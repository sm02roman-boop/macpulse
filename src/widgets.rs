use eframe::egui::{self, epaint, pos2, vec2, Color32, Pos2, RichText, Sense, Shape, Stroke, Ui};
use std::collections::VecDeque;

pub fn card<R>(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(crate::theme::card_fill())
        .stroke(Stroke::new(1.0, crate::theme::card_border()))
        .corner_radius(16)
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                ui.label(RichText::new(title).strong().color(crate::theme::ACCENT_SOFT));
                ui.add_space(6.0);
            }
            add(ui)
        })
        .inner
}

pub fn sparkline(ui: &mut Ui, vals: &VecDeque<f32>, max: Option<f32>, color: Color32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 4.0, color.gamma_multiply(0.06));
    if vals.len() < 2 {
        return;
    }
    let mx = max.unwrap_or_else(|| vals.iter().cloned().fold(1.0, f32::max) * 1.15);
    let step = rect.width() / (crate::monitor::HISTORY - 1) as f32;
    let n = vals.len();
    let pts: Vec<Pos2> = vals
        .iter()
        .enumerate()
        .map(|(i, v)| {
            pos2(
                rect.right() - (n - 1 - i) as f32 * step,
                rect.bottom() - (v / mx).clamp(0.0, 1.0) * rect.height(),
            )
        })
        .collect();
    let mut mesh = epaint::Mesh::default();
    let fill = color.gamma_multiply(0.22);
    for w in pts.windows(2) {
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(w[0], fill);
        mesh.colored_vertex(w[1], fill);
        mesh.colored_vertex(pos2(w[1].x, rect.bottom()), Color32::TRANSPARENT);
        mesh.colored_vertex(pos2(w[0].x, rect.bottom()), Color32::TRANSPARENT);
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    }
    p.add(Shape::mesh(mesh));
    p.add(Shape::line(pts.clone(), Stroke::new(6.0_f32, color.gamma_multiply(0.12))));
    p.add(Shape::line(pts, Stroke::new(1.8_f32, color)));
}

pub fn ring(ui: &mut Ui, frac: f32, label: &str, color: Color32) {
    let size = 84.0;
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let p = ui.painter_at(rect);
    let c = rect.center();
    let r = size / 2.0 - 7.0;
    let arc = |from: f32, to: f32| -> Vec<Pos2> {
        let steps = 64;
        (0..=steps)
            .map(|i| {
                let a = from + (to - from) * i as f32 / steps as f32;
                pos2(c.x + r * a.cos(), c.y + r * a.sin())
            })
            .collect()
    };
    let start = -std::f32::consts::FRAC_PI_2;
    let full = std::f32::consts::TAU;
    p.add(Shape::line(arc(start, start + full), Stroke::new(8.0_f32, color.gamma_multiply(0.15))));
    let f = frac.clamp(0.0, 1.0);
    if f > 0.002 {
        p.add(Shape::line(arc(start, start + full * f), Stroke::new(15.0_f32, color.gamma_multiply(0.14))));
        p.add(Shape::line(arc(start, start + full * f), Stroke::new(8.0_f32, color)));
    }
    p.text(c, egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(17.0), ui.visuals().strong_text_color());
}

pub fn level_color(pct: f32) -> Color32 {
    if pct >= 85.0 {
        crate::theme::BAD
    } else if pct >= 65.0 {
        crate::theme::WARN
    } else {
        crate::theme::GOOD
    }
}

pub fn header(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(27.0).strong());
    ui.label(RichText::new(subtitle).color(crate::theme::MUTED));
    ui.add_space(6.0);
}

pub const BLUE: Color32 = Color32::from_rgb(70, 140, 235);
pub const PURPLE: Color32 = Color32::from_rgb(150, 110, 230);
pub const TEAL: Color32 = Color32::from_rgb(60, 180, 190);
pub const ORANGE: Color32 = Color32::from_rgb(235, 140, 60);

pub fn bar(ui: &mut Ui, frac: f32, text: String, color: Color32) {
    let h = 24.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, h / 2.0, crate::theme::veil(14));
    let frac = frac.clamp(0.0, 1.0);
    if frac > 0.0 {
        let w = (rect.width() * frac).max(h).min(rect.width());
        let fill = egui::Rect::from_min_size(rect.min, vec2(w, h));
        p.rect_filled(fill, h / 2.0, color.gamma_multiply(0.85));
        p.rect_filled(egui::Rect::from_min_size(fill.min + vec2(3.0, 2.5), vec2((w - 8.0).max(0.0), 5.0)), 3.0, crate::theme::veil(22));
    }
    let font = egui::FontId::proportional(12.5);
    p.text(rect.left_center() + vec2(11.0, 1.0), egui::Align2::LEFT_CENTER, &text, font.clone(), Color32::from_black_alpha(150));
    p.text(rect.left_center() + vec2(11.0, 0.0), egui::Align2::LEFT_CENTER, text, font, Color32::WHITE);
}

/// Accent button with a glossy top highlight.
pub fn primary(ui: &mut Ui, text: &str) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), egui::FontId::proportional(14.0), Color32::WHITE);
    let (rect, resp) = ui.allocate_exact_size(galley.size() + vec2(30.0, 14.0), Sense::click());
    let base = if resp.hovered() { crate::theme::ACCENT.gamma_multiply(1.15) } else { crate::theme::ACCENT };
    let p = ui.painter();
    p.rect_filled(rect, 10.0, base);
    p.rect_filled(egui::Rect::from_min_size(rect.min + vec2(2.0, 2.0), vec2(rect.width() - 4.0, 6.0)), 4.0, crate::theme::veil(26));
    p.galley(rect.center() - galley.size() / 2.0, galley, Color32::WHITE);
    resp
}
