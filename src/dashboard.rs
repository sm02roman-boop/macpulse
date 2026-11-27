//! The Dashboard page: machine card, four live vitals cards, Smart Care banner.

use crate::icons::{self, Icon};
use crate::monitor::{Machine, MonState, Severity};
use crate::widgets::{card, level_color, pill, primary_with, sparkline, BLUE, ORANGE, PURPLE, TEAL};
use crate::{theme, util};
use eframe::egui::{self, vec2, Color32, RichText, Sense, Ui};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Go {
    Memory,
    Cpu,
    Storage,
    Battery,
    Network,
    SmartCare,
    Assistant,
    Settings,
}

pub struct CareSummary {
    pub total: u64,
    pub safe: u64,
    pub categories: usize,
}

pub fn status(m: &MonState) -> (&'static str, Color32) {
    if m.findings.iter().any(|f| f.severity == Severity::Critical) {
        ("Critical", theme::BAD)
    } else if m.findings.iter().any(|f| f.severity == Severity::Warn) {
        ("Needs attention", theme::WARN)
    } else {
        ("Optimal", theme::GOOD)
    }
}

pub fn uptime_text(secs: u64) -> String {
    let (d, h, m) = (secs / 86400, secs % 86400 / 3600, secs % 3600 / 60);
    if d > 0 { format!("{d}d {h}h") } else if h > 0 { format!("{h}h {m:02}m") } else { format!("{m}m") }
}

/// Status line shown at the right of the page title. Call it inside a right-to-left layout:
/// items are added in reverse visual order.
pub fn status_bar(ui: &mut Ui, m: &MonState) {
    let (text, color) = status(m);
    ui.label(RichText::new(format!("Up {}", uptime_text(m.snap.uptime))).color(theme::MUTED));
    ui.label(RichText::new("·").color(theme::MUTED));
    ui.label(RichText::new(format!("System status: {text}")).strong());
    let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
    ui.painter().circle_filled(r.center(), 4.5, color);
    ui.painter().circle_filled(r.center(), 8.0, color.gamma_multiply(0.18));
}

fn chip(ui: &mut Ui, text: String) {
    egui::Frame::new().fill(theme::veil(16)).corner_radius(9).inner_margin(egui::Margin::symmetric(10, 7)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(text).small().color(Color32::from_rgb(214, 214, 232)));
    });
}

fn chips(ui: &mut Ui, a: String, b: String) {
    ui.columns(2, |c| {
        chip(&mut c[0], a);
        chip(&mut c[1], b);
    });
}

fn link(ui: &mut Ui, text: &str, color: Color32) -> bool {
    let r = ui.add(egui::Label::new(RichText::new(format!("{text}  >")).strong().color(color)).sense(Sense::click()));
    r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn header(ui: &mut Ui, icon: Icon, tint: Color32, title: &str, status: (String, Color32)) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
        icons::tile(ui.painter(), icon, r.center(), 32.0, tint);
        ui.label(RichText::new(title).size(16.5).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| pill(ui, &status.0, status.1));
    });
}

fn big_line(ui: &mut Ui, big: &str, small: &str, small_color: Color32, badge: Option<(String, Color32)>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(big).size(30.0).strong());
        ui.label(RichText::new(small).size(14.0).color(small_color));
        if let Some((t, c)) = badge {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| pill(ui, &t, c));
        }
    });
}

fn level_label(pct: f32) -> (String, Color32) {
    if pct >= 90.0 {
        ("CRITICAL".into(), theme::BAD)
    } else if pct >= 75.0 {
        ("WARNING".into(), theme::WARN)
    } else {
        ("NORMAL".into(), theme::GOOD)
    }
}

pub fn storage_visual(ui: &mut Ui, name: &str, total: u64, free: u64, reclaim: u64) {
    let used = total.saturating_sub(free);
    ui.horizontal(|ui| {
        ui.label(RichText::new(if name.is_empty() { "Startup disk" } else { name }).size(15.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(format!("{} of {} used", util::size(used), util::size(total))).color(theme::MUTED));
        });
    });
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 9.0, theme::veil(16));
    if total > 0 {
        let frac = used as f32 / total as f32;
        let used_w = (rect.width() * frac).clamp(0.0, rect.width());
        let used_rect = egui::Rect::from_min_size(rect.min, vec2(used_w, rect.height()));
        p.rect_filled(used_rect, 9.0, Color32::from_rgb(110, 120, 156));
        let rec_w = (rect.width() * reclaim as f32 / total as f32).min(used_w);
        if rec_w > 2.0 {
            p.rect_filled(egui::Rect::from_min_size(used_rect.right_top() - vec2(rec_w, 0.0), vec2(rec_w, rect.height())), 9.0, BLUE);
        }
        p.text(rect.right_center() - vec2(10.0, 0.0), egui::Align2::RIGHT_CENTER, util::size(free), egui::FontId::proportional(13.0), Color32::WHITE);
    }
    ui.horizontal(|ui| {
        for (c, t) in [
            (Color32::from_rgb(110, 120, 156), "Used".to_string()),
            (BLUE, format!("Smart Care can free {}", util::size(reclaim))),
            (Color32::from_white_alpha(70), "Free".to_string()),
        ] {
            let (r, _) = ui.allocate_exact_size(vec2(9.0, 9.0), Sense::hover());
            ui.painter().circle_filled(r.center(), 4.0, c);
            ui.label(RichText::new(t).small().color(theme::MUTED));
        }
    });
}

pub fn show(ui: &mut Ui, m: &MonState, mach: &Machine, care: &CareSummary) -> Option<Go> {
    let s = &m.snap;
    let mut go = None;

    // Machine card
    card(ui, "", |ui| {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(60.0, 60.0), Sense::hover());
            icons::tile(ui.painter(), Icon::Laptop, r.center(), 60.0, Color32::from_rgb(40, 100, 200));
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&mach.name).size(24.0).strong());
                    pill(ui, &mach.os.to_uppercase(), theme::ACCENT_SOFT);
                });
                let thermal = match s.speed_limit {
                    Some(l) if l < 100 => (format!("Throttled to {l}%"), theme::WARN),
                    _ => ("Thermals normal".to_string(), theme::GOOD),
                };
                ui.horizontal_wrapped(|ui| {
                    if !mach.cpu.is_empty() {
                        ui.label(RichText::new(&mach.cpu).color(theme::MUTED));
                        ui.label(RichText::new("·").color(theme::MUTED));
                    }
                    ui.label(RichText::new(format!("{} RAM", util::bytes(mach.ram))).color(theme::MUTED));
                    ui.label(RichText::new("·").color(theme::MUTED));
                    ui.label(RichText::new(thermal.0).strong().color(thermal.1));
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::Frame::new().fill(theme::veil(14)).corner_radius(10).inner_margin(egui::Margin::symmetric(14, 8)).show(ui, |ui| {
                    ui.set_width(56.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(m.groups.len().to_string()).size(20.0).strong());
                        ui.label(RichText::new("Apps").small().color(theme::MUTED));
                    });
                });
                egui::Frame::new().fill(theme::veil(14)).corner_radius(10).inner_margin(egui::Margin::symmetric(14, 8)).show(ui, |ui| {
                    ui.set_width(150.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("Down  {}", util::rate(s.rx_rate))).strong().color(TEAL));
                        ui.label(RichText::new(format!("Up      {}", util::rate(s.tx_rate))).strong().color(ORANGE));
                    });
                });
            });
        });
    });

    ui.add_space(14.0);
    ui.label(RichText::new("Real-time system vitals").size(15.0).strong().color(theme::MUTED));
    ui.add_space(4.0);

    let mem_pct = if s.mem_total > 0 { s.mem_used as f32 / s.mem_total as f32 * 100.0 } else { 0.0 };
    let peak = m.cpu.iter().cloned().fold(0.0_f32, f32::max);
    let top_cpu = m.groups.iter().max_by(|a, b| a.cpu.total_cmp(&b.cpu));
    let pressure = match s.pressure { 4.. => "Critical", 2..=3 => "Elevated", _ => "Normal" };

    ui.columns(2, |c| {
        // Memory
        card(&mut c[0], "", |ui| {
            ui.set_min_height(236.0);
            let (lt, lc) = level_label(if s.pressure >= 4 { 95.0 } else if s.pressure >= 2 { 80.0 } else { mem_pct.min(74.0) });
            header(ui, Icon::Memory, PURPLE, "Memory (RAM)", (lt, lc));
            big_line(ui, &util::bytes(s.mem_used), &format!("/ {}", util::bytes(s.mem_total)), theme::MUTED, Some((format!("{mem_pct:.0}%"), level_color(mem_pct))));
            sparkline(ui, &m.mem, Some(100.0), PURPLE, 84.0);
            chips(ui, format!("Swap: {}", util::bytes(s.swap_used)), format!("Pressure: {pressure}"));
            if link(ui, "Inspect Memory Treemap", PURPLE) {
                go = Some(Go::Memory);
            }
        });
        // CPU
        card(&mut c[1], "", |ui| {
            ui.set_min_height(236.0);
            let (lt, lc) = level_label(s.cpu_total);
            header(ui, Icon::Cpu, BLUE, "Processor (CPU)", (lt, lc));
            big_line(ui, &format!("{:.0}%", s.cpu_total), "System load", theme::MUTED, Some((format!("Peak {peak:.0}%"), theme::MUTED)));
            sparkline(ui, &m.cpu, Some(100.0), BLUE, 84.0);
            chips(ui, format!("Cores: {}", s.cores.len()), match top_cpu { Some(g) => format!("Top: {} {:.0}%", g.name, g.cpu), None => "Top: -".into() });
            if link(ui, "Inspect CPU Cores", BLUE) {
                go = Some(Go::Cpu);
            }
        });
    });
    ui.add_space(8.0);
    ui.columns(2, |c| {
        // Storage
        card(&mut c[0], "", |ui| {
            ui.set_min_height(236.0);
            let free_pct = if s.disk_total > 0 { s.disk_free as f32 / s.disk_total as f32 * 100.0 } else { 100.0 };
            let st = if free_pct < 5.0 { ("CRITICAL".to_string(), theme::BAD) } else if free_pct < 10.0 { ("LOW".to_string(), theme::WARN) } else { ("NORMAL".to_string(), theme::GOOD) };
            header(ui, Icon::Storage, Color32::from_rgb(60, 130, 220), "Storage & disk map", st);
            ui.add_space(10.0);
            storage_visual(ui, &s.disk_name, s.disk_total, s.disk_free, care.safe);
            ui.add_space(12.0);
            if link(ui, "Explore Visual Treemap", Color32::from_rgb(90, 160, 240)) {
                go = Some(Go::Storage);
            }
        });
        // Battery (or network on desktops)
        match &s.battery {
            Some(b) => card(&mut c[1], "", |ui| {
                ui.set_min_height(236.0);
                let low = b.percent < 20.0 && !b.charging;
                let st = if low { ("CRITICAL".to_string(), theme::BAD) } else if b.health_percent.map(|h| h < 80.0).unwrap_or(false) { ("WORN".to_string(), theme::WARN) } else { ("NORMAL".to_string(), theme::GOOD) };
                header(ui, Icon::Battery, Color32::from_rgb(52, 170, 110), "Battery", st);
                let state = if b.charging { "Charging" } else if b.plugged_in { "Plugged in" } else { "On battery" };
                big_line(ui, &format!("{:.0}%", b.percent), state, theme::GOOD, b.health_percent.map(|h| (format!("{h:.0}% health"), if h < 80.0 { theme::WARN } else { theme::GOOD })));
                crate::widgets::bar(ui, b.percent / 100.0, String::new(), theme::GOOD);
                ui.add_space(4.0);
                let time = match b.minutes_remaining {
                    Some(mn) => format!("{} {}h {:02}m", if b.charging { "To full:" } else { "Left:" }, mn / 60, mn % 60),
                    None => "Time: calculating".to_string(),
                };
                chips(ui, time, format!("Cycles: {}", b.cycles.map(|c| c.to_string()).unwrap_or_else(|| "-".into())));
                if link(ui, "Open Battery Insights", theme::GOOD) {
                    go = Some(Go::Battery);
                }
            }),
            None => card(&mut c[1], "", |ui| {
                ui.set_min_height(236.0);
                header(ui, Icon::Network, TEAL, "Network", ("ACTIVE".to_string(), theme::GOOD));
                big_line(ui, &util::rate(s.rx_rate), "download", theme::MUTED, None);
                sparkline(ui, &m.rx, None, TEAL, 84.0);
                chips(ui, format!("Upload: {}", util::rate(s.tx_rate)), format!("Apps: {}", m.groups.len()));
                if link(ui, "Open Network", TEAL) {
                    go = Some(Go::Network);
                }
            }),
        }
    });

    // Smart Care banner
    ui.add_space(10.0);
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(40, 190, 140, 30))
        .stroke(egui::Stroke::new(1.0_f32, theme::GOOD.gamma_multiply(0.5)))
        .corner_radius(16)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
                icons::tile(ui.painter(), Icon::SmartCare, r.center(), 48.0, Color32::from_rgb(34, 170, 130));
                ui.vertical(|ui| {
                    ui.label(RichText::new("Smart Care").size(20.0).strong());
                    ui.label(RichText::new(format!("{} safe to free now · {} categories", util::size(care.safe), care.categories)).color(theme::MUTED));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if primary_with(ui, "Reclaim", Color32::from_rgb(30, 160, 150)).clicked() {
                        go = Some(Go::SmartCare);
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(util::size(care.total)).size(26.0).strong().color(theme::GOOD));
                        ui.label(RichText::new("can be freed").small().color(theme::MUTED));
                    });
                });
            });
        });
    go
}
