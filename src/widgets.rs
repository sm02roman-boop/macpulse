use eframe::egui::{self, epaint, pos2, vec2, Color32, Pos2, RichText, Sense, Shape, Stroke, Ui};
use std::collections::VecDeque;

pub fn card<R>(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::group(ui.style())
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).strong());
            ui.add_space(6.0);
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
    p.add(Shape::line(pts, Stroke::new(1.5_f32, color)));
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
        p.add(Shape::line(arc(start, start + full * f), Stroke::new(8.0_f32, color)));
    }
    p.text(c, egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(17.0), ui.visuals().strong_text_color());
}

pub fn level_color(pct: f32) -> Color32 {
    if pct >= 85.0 {
        Color32::from_rgb(232, 90, 79)
    } else if pct >= 65.0 {
        Color32::from_rgb(235, 170, 60)
    } else {
        Color32::from_rgb(80, 170, 120)
    }
}

pub const BLUE: Color32 = Color32::from_rgb(70, 140, 235);
pub const PURPLE: Color32 = Color32::from_rgb(150, 110, 230);
pub const TEAL: Color32 = Color32::from_rgb(60, 180, 190);
pub const ORANGE: Color32 = Color32::from_rgb(235, 140, 60);

pub fn bar(ui: &mut Ui, frac: f32, text: String, color: Color32) {
    ui.add(egui::ProgressBar::new(frac.clamp(0.0, 1.0)).fill(color).text(text));
}
