use eframe::egui::{self, vec2, Color32, CornerRadius, FontId, Painter, Pos2, Stroke, TextStyle};
use std::sync::atomic::{AtomicBool, Ordering};

static GLASS: AtomicBool = AtomicBool::new(false);

pub fn set_glass(on: bool) {
    GLASS.store(on, Ordering::Relaxed);
}

/// Translucent white for fills and strokes. Measured: the renderer shows plain `from_white_alpha(n)`
/// about 4x stronger than n/255, so scale it down to get the intended subtle frosted look.
pub fn veil(a: u8) -> Color32 {
    Color32::from_white_alpha(((a as f32) * 0.26).round() as u8)
}

pub fn glass() -> bool {
    GLASS.load(Ordering::Relaxed)
}

/// Frosted card: a faint white veil over whatever is behind. Opaque when glass is off.
pub fn card_fill() -> Color32 {
    if glass() { veil(14) } else { CARD }
}

pub fn card_border() -> Color32 {
    if glass() { veil(34) } else { BORDER }
}

pub fn sidebar_fill() -> Color32 {
    if glass() { Color32::from_rgb(11, 11, 19) } else { BG }
}

pub fn content_fill() -> Color32 {
    if glass() { Color32::from_rgb(17, 17, 27) } else { PANEL }
}

pub fn ai_fill() -> Color32 {
    if glass() { Color32::from_rgba_unmultiplied(139, 92, 246, 46) } else { Color32::from_rgb(38, 30, 62) }
}

/// Space reserved at the top for the macOS traffic-light buttons (title bar is merged into the window).
pub fn top_inset() -> i8 {
    if cfg!(target_os = "macos") { 26 } else { 0 }
}

fn glow(p: &Painter, center: Pos2, radius: f32, color: Color32) {
    let n = 48;
    let mut m = egui::Mesh::default();
    m.colored_vertex(center, color);
    for i in 0..=n {
        let a = std::f32::consts::TAU * i as f32 / n as f32;
        m.colored_vertex(center + vec2(a.cos(), a.sin()) * radius, Color32::TRANSPARENT);
    }
    for i in 0..n {
        m.add_triangle(0, 1 + i, 2 + i);
    }
    p.add(egui::Shape::mesh(m));
}

/// Soft coloured light behind the content, so the frosted cards have something to refract.
pub fn backdrop(p: &Painter, r: egui::Rect) {
    glow(p, r.right_top() + vec2(-80.0, 40.0), 460.0, Color32::from_rgba_unmultiplied(139, 92, 246, 60));
    glow(p, r.left_bottom() + vec2(120.0, -20.0), 400.0, Color32::from_rgba_unmultiplied(56, 120, 246, 42));
}

pub const BG: Color32 = Color32::from_rgb(15, 15, 22);
pub const PANEL: Color32 = Color32::from_rgb(21, 21, 31);
pub const CARD: Color32 = Color32::from_rgb(29, 29, 42);
pub const BORDER: Color32 = Color32::from_rgb(52, 52, 74);
pub const ACCENT: Color32 = Color32::from_rgb(139, 92, 246);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(196, 170, 255);
pub const MUTED: Color32 = Color32::from_rgb(150, 150, 176);
pub const GOOD: Color32 = Color32::from_rgb(74, 190, 130);
pub const WARN: Color32 = Color32::from_rgb(240, 175, 60);
pub const BAD: Color32 = Color32::from_rgb(236, 92, 82);

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = content_fill();
    v.window_fill = Color32::from_rgba_unmultiplied(30, 30, 46, 238);
    v.extreme_bg_color = if glass() { Color32::from_black_alpha(100) } else { BG };
    v.faint_bg_color = card_fill();
    v.window_stroke = Stroke::new(1.0_f32, card_border());
    v.window_corner_radius = CornerRadius::same(14);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.5);
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.hyperlink_color = ACCENT_SOFT;
    let r = CornerRadius::same(9);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = r;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, card_border());
    let btn = if glass() { veil(20) } else { Color32::from_rgb(44, 44, 62) };
    let btn_hover = if glass() { veil(38) } else { Color32::from_rgb(62, 56, 94) };
    v.widgets.inactive.bg_fill = btn;
    v.widgets.inactive.weak_bg_fill = btn;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, veil(if glass() { 26 } else { 0 }));
    v.widgets.hovered.bg_fill = btn_hover;
    v.widgets.hovered.weak_bg_fill = btn_hover;
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.weak_bg_fill = ACCENT;
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(9.0, 8.0);
        s.spacing.button_padding = egui::vec2(13.0, 6.0);
        s.spacing.interact_size.y = 28.0;
        s.text_styles.insert(TextStyle::Heading, FontId::proportional(24.0));
        s.text_styles.insert(TextStyle::Body, FontId::proportional(14.5));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(14.0));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(12.0));
    });
}
