use eframe::egui::{self, Color32, CornerRadius, FontId, Stroke, TextStyle};

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
    v.panel_fill = PANEL;
    v.window_fill = CARD;
    v.extreme_bg_color = BG;
    v.faint_bg_color = CARD;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_corner_radius = CornerRadius::same(14);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.5);
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
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
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.bg_fill = Color32::from_rgb(44, 44, 62);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(44, 44, 62);
    v.widgets.hovered.bg_fill = Color32::from_rgb(62, 56, 94);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(62, 56, 94);
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
