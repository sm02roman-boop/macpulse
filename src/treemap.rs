use crate::util::hue;
use eframe::egui::{self, ecolor::Hsva, vec2, Align2, Color32, FontId, Rect, Sense, Stroke, Ui};

pub struct Item {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub value: f64,
}

pub struct Click {
    pub id: String,
    pub double: bool,
}

/// Squarified treemap layout (Bruls, Huijing, van Wijk). `values` must be sorted descending.
pub fn squarify(values: &[f64], bounds: Rect) -> Vec<Rect> {
    let total: f64 = values.iter().sum();
    let mut out = vec![Rect::NOTHING; values.len()];
    if total <= 0.0 || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return out;
    }
    let scale = (bounds.width() * bounds.height()) as f64 / total;
    let areas: Vec<f64> = values.iter().map(|v| v * scale).collect();
    let worst = |row: &[f64], side: f64| -> f64 {
        let s: f64 = row.iter().sum();
        let mx = row.iter().cloned().fold(f64::MIN, f64::max);
        let mn = row.iter().cloned().fold(f64::MAX, f64::min);
        (side * side * mx / (s * s)).max(s * s / (side * side * mn))
    };
    let mut rem = bounds;
    let mut i = 0;
    while i < areas.len() {
        if rem.width() <= 0.5 || rem.height() <= 0.5 {
            break;
        }
        let side = rem.width().min(rem.height()) as f64;
        let mut end = i + 1;
        let mut best = worst(&areas[i..end], side);
        while end < areas.len() {
            let w = worst(&areas[i..end + 1], side);
            if w <= best {
                best = w;
                end += 1;
            } else {
                break;
            }
        }
        let row = &areas[i..end];
        let sum: f64 = row.iter().sum();
        if rem.width() >= rem.height() {
            let col_w = (sum / rem.height() as f64) as f32;
            let mut y = rem.top();
            for (k, a) in row.iter().enumerate() {
                let h = (*a as f32) / col_w;
                out[i + k] = Rect::from_min_size(egui::pos2(rem.left(), y), vec2(col_w, h));
                y += h;
            }
            rem = Rect::from_min_max(egui::pos2(rem.left() + col_w, rem.top()), rem.max);
        } else {
            let row_h = (sum / rem.width() as f64) as f32;
            let mut x = rem.left();
            for (k, a) in row.iter().enumerate() {
                let w = (*a as f32) / row_h;
                out[i + k] = Rect::from_min_size(egui::pos2(x, rem.top()), vec2(w, row_h));
                x += w;
            }
            rem = Rect::from_min_max(egui::pos2(rem.left(), rem.top() + row_h), rem.max);
        }
        i = end;
    }
    out
}

pub fn show(ui: &mut Ui, items: &[Item], selected: Option<&str>, height: f32) -> Option<Click> {
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let mut sorted: Vec<&Item> = items.iter().filter(|i| i.value > 0.0).collect();
    sorted.sort_by(|a, b| b.value.total_cmp(&a.value));
    let values: Vec<f64> = sorted.iter().map(|i| i.value).collect();
    let rects = squarify(&values, area);
    let mut clicked = None;
    for (item, r) in sorted.iter().zip(rects) {
        let r = r.shrink(1.0);
        if r.width() < 2.0 || r.height() < 2.0 {
            continue;
        }
        let resp = ui.interact(r, ui.id().with(&item.id), Sense::click());
        let base: Color32 = Hsva::new(hue(&item.label), 0.55, 0.52, 1.0).into();
        let is_sel = selected == Some(item.id.as_str());
        let fill = if resp.hovered() { base.gamma_multiply(1.2) } else { base };
        let p = ui.painter().with_clip_rect(r);
        p.rect_filled(r, 6.0, fill);
        ui.painter().rect_stroke(r, 6.0, Stroke::new(1.0_f32, crate::theme::veil(44)), egui::StrokeKind::Inside);
        if is_sel {
            ui.painter().rect_stroke(r, 4.0, Stroke::new(2.5_f32, Color32::WHITE), egui::StrokeKind::Inside);
        }
        if r.width() > 64.0 && r.height() > 32.0 {
            p.text(r.left_top() + vec2(6.0, 5.0), Align2::LEFT_TOP, &item.label, FontId::proportional(12.5), Color32::WHITE);
            p.text(r.left_top() + vec2(6.0, 21.0), Align2::LEFT_TOP, &item.detail, FontId::proportional(11.0), Color32::from_white_alpha(200));
        }
        resp.clone().on_hover_text(format!("{}\n{}", item.label, item.detail));
        if resp.clicked() || resp.double_clicked() {
            clicked = Some(Click { id: item.id.clone(), double: resp.double_clicked() });
        }
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn areas_are_proportional_and_inside_bounds() {
        let b = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(600.0, 400.0));
        let v = [6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0];
        let r = squarify(&v, b);
        let total: f64 = v.iter().sum();
        for (val, rect) in v.iter().zip(&r) {
            let want = val / total * (600.0 * 400.0);
            assert!((rect.area() as f64 - want).abs() < want * 0.01, "{rect:?}");
            assert!(b.expand(0.01).contains_rect(*rect));
        }
    }
}
