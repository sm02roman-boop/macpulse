//! Menu-bar popover: a compact floating dashboard that opens under the menu bar item.

use crate::dashboard::{self, CareSummary, Go};
use crate::icons::{self, Icon};
use crate::monitor::{Machine, MonState};
use crate::widgets::{self, level_color, pill, sparkline, BLUE, PURPLE, TEAL};
use crate::{theme, util};
use eframe::egui::{self, vec2, Color32, RichText, Sense, Ui};

pub const SIZE: [f32; 2] = [392.0, 700.0];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Memory,
    Disk,
    Network,
    Battery,
}

pub enum Action {
    Go(Go),
    Quit,
}

pub struct State {
    pub open: bool,
    pub tab: Tab,
    pub pos: egui::Pos2,
    pub focused_once: bool,
}

impl State {
    pub fn new() -> Self {
        Self { open: false, tab: Tab::Overview, pos: egui::pos2(100.0, 30.0), focused_once: false }
    }
}

fn round_button(ui: &mut Ui, icon: Icon, tint: Color32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
    ui.painter().circle_filled(rect.center(), 17.0, if resp.hovered() { theme::veil(70) } else { theme::veil(42) });
    icons::draw(ui.painter(), icon, rect.center(), tint);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn tabs(ui: &mut Ui, tab: &mut Tab) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
    ui.painter().rect_filled(rect, 14.0, theme::veil(34));
    let items = [(Tab::Overview, Icon::Overview), (Tab::Memory, Icon::Memory), (Tab::Disk, Icon::Storage), (Tab::Network, Icon::Network), (Tab::Battery, Icon::Battery)];
    let w = rect.width() / items.len() as f32;
    for (i, (t, icon)) in items.into_iter().enumerate() {
        let r = egui::Rect::from_min_size(rect.min + vec2(w * i as f32, 0.0), vec2(w, rect.height())).shrink2(vec2(3.0, 4.0));
        let resp = ui.interact(r, ui.id().with(("tab", i)), Sense::click());
        if *tab == t {
            ui.painter().rect_filled(r, 11.0, theme::ACCENT.gamma_multiply(0.35));
            ui.painter().rect_stroke(r, 11.0, egui::Stroke::new(1.0_f32, theme::ACCENT.gamma_multiply(0.7)), egui::StrokeKind::Inside);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, 11.0, theme::veil(40));
        }
        icons::draw(ui.painter(), icon, r.center(), if *tab == t { Color32::WHITE } else { Color32::from_rgb(180, 180, 205) });
        if resp.clicked() {
            *tab = t;
        }
    }
}

fn tile(ui: &mut Ui, title: &str, dot: Color32, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new().fill(theme::card_fill()).stroke(egui::Stroke::new(1.0_f32, theme::card_border())).corner_radius(14).inner_margin(10).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(104.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).small().strong().color(theme::MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 4.0, dot);
            });
        });
        add(ui);
    });
}

fn app_rows(ui: &mut Ui, m: &MonState, n: usize) {
    for g in m.groups.iter().take(n) {
        egui::Frame::new().fill(theme::veil(26)).corner_radius(10).inner_margin(egui::Margin::symmetric(12, 8)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(&g.name).size(14.5));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(util::bytes(g.rss)).monospace().color(theme::MUTED)));
            });
        });
    }
}

fn white_button(ui: &mut Ui, text: &str, fg: Color32) -> bool {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), egui::FontId::proportional(15.0), fg);
    let (rect, resp) = ui.allocate_exact_size(galley.size() + vec2(34.0, 18.0), Sense::click());
    ui.painter().rect_filled(rect, rect.height() / 2.0, if resp.hovered() { Color32::WHITE } else { Color32::from_rgb(244, 248, 246) });
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn smart_care_card(ui: &mut Ui, care: &CareSummary, cats: &[String]) -> bool {
    let mut clicked = false;
    egui::Frame::new().fill(Color32::from_rgb(16, 112, 92)).stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(40, 170, 140))).corner_radius(18).inner_margin(16).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(42.0, 42.0), Sense::hover());
            icons::tile(ui.painter(), Icon::SmartCare, r.center(), 42.0, Color32::from_rgb(60, 170, 146));
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Smart Care").size(19.0).strong().color(Color32::WHITE));
                    pill(ui, "FREE", Color32::WHITE);
                });
                ui.label(RichText::new("Deep clean caches and junk").color(Color32::from_rgb(200, 235, 225)));
            });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(util::size(care.total)).size(30.0).strong().color(Color32::WHITE));
                ui.label(RichText::new("recoverable junk found").color(Color32::from_rgb(200, 235, 225)));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                clicked = white_button(ui, "Clean", Color32::from_rgb(14, 100, 80));
            });
        });
        ui.horizontal_wrapped(|ui| {
            for c in cats.iter().take(3) {
                egui::Frame::new().fill(Color32::from_white_alpha(40)).corner_radius(10).inner_margin(egui::Margin::symmetric(9, 3)).show(ui, |ui| {
                    ui.label(RichText::new(c).small().color(Color32::WHITE));
                });
            }
        });
    });
    clicked
}

fn overview(ui: &mut Ui, m: &MonState, care: &CareSummary, cats: &[String]) -> Option<Action> {
    let s = &m.snap;
    let mut act = None;
    if smart_care_card(ui, care, cats) {
        act = Some(Action::Go(Go::SmartCare));
    }
    ui.add_space(8.0);
    egui::Frame::new().fill(Color32::from_rgb(40, 40, 62)).corner_radius(14).inner_margin(12).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Ask AI about your Mac").strong().size(15.0));
                ui.label(RichText::new("Plain-language answers, free on this Mac").small().color(theme::MUTED));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::primary(ui, "Open").clicked() {
                    act = Some(Action::Go(Go::Assistant));
                }
            });
        });
    });
    ui.add_space(8.0);
    let mem_pct = if s.mem_total > 0 { s.mem_used as f32 / s.mem_total as f32 * 100.0 } else { 0.0 };
    let disk_used_pct = if s.disk_total > 0 { 100.0 - s.disk_free as f32 / s.disk_total as f32 * 100.0 } else { 0.0 };
    ui.columns(3, |c| {
        tile(&mut c[0], "Memory", level_color(mem_pct), |ui| {
            ui.label(RichText::new(util::bytes(s.mem_used)).size(17.0).strong());
            ui.label(RichText::new(format!("of {}", util::bytes(s.mem_total))).small().color(theme::MUTED));
            sparkline(ui, &m.mem, Some(100.0), Color32::from_rgb(60, 190, 120), 34.0);
        });
        tile(&mut c[1], "CPU", level_color(s.cpu_total), |ui| {
            ui.label(RichText::new(format!("{:.0}%", s.cpu_total)).size(17.0).strong());
            ui.label(RichText::new("Load").small().color(theme::MUTED));
            sparkline(ui, &m.cpu, Some(100.0), Color32::from_rgb(240, 160, 50), 34.0);
        });
        tile(&mut c[2], "Disk", level_color(disk_used_pct), |ui| {
            ui.label(RichText::new(util::size(s.disk_free)).size(17.0).strong());
            ui.label(RichText::new("Free").small().color(theme::MUTED));
            widgets::bar(ui, disk_used_pct / 100.0, String::new(), level_color(disk_used_pct));
        });
    });
    if let Some(b) = &s.battery {
        ui.add_space(8.0);
        egui::Frame::new().fill(theme::card_fill()).stroke(egui::Stroke::new(1.0_f32, theme::card_border())).corner_radius(14).inner_margin(12).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("Battery").strong());
                ui.label(RichText::new(format!("{:.0}%", b.percent)).size(18.0).strong().color(theme::GOOD));
                ui.label(RichText::new(if b.charging { "Charging" } else if b.plugged_in { "Plugged in" } else { "On battery" }).color(theme::GOOD));
                if let Some(h) = b.health_percent {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| pill(ui, &format!("{h:.0}% health"), if h < 80.0 { theme::BAD } else { theme::GOOD }));
                }
            });
            widgets::bar(ui, b.percent / 100.0, String::new(), theme::GOOD);
            ui.horizontal(|ui| {
                if let Some(mn) = b.minutes_remaining {
                    ui.label(RichText::new(format!("{}h {:02}m {}", mn / 60, mn % 60, if b.charging { "to full" } else { "left" })).small().color(theme::MUTED));
                }
                if let Some(c) = b.cycles {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(format!("{c} cycles")).small().color(theme::MUTED)));
                }
            });
        });
    }
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Top memory apps").size(15.0).strong().color(theme::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(egui::Label::new(RichText::new("View all").strong().color(BLUE)).sense(Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                act = Some(Action::Go(Go::Memory));
            }
        });
    });
    app_rows(ui, m, 4);
    act
}

fn memory_tab(ui: &mut Ui, m: &MonState) -> Option<Action> {
    let s = &m.snap;
    ui.label(RichText::new(format!("{} of {} used", util::bytes(s.mem_used), util::bytes(s.mem_total))).size(18.0).strong());
    ui.label(RichText::new(format!("Swap {} · pressure {}", util::bytes(s.swap_used), match s.pressure { 4.. => "critical", 2..=3 => "elevated", _ => "normal" })).color(theme::MUTED));
    sparkline(ui, &m.mem, Some(100.0), PURPLE, 90.0);
    ui.add_space(8.0);
    app_rows(ui, m, 9);
    None
}

fn disk_tab(ui: &mut Ui, m: &MonState, care: &CareSummary) -> Option<Action> {
    let s = &m.snap;
    dashboard::storage_visual(ui, &s.disk_name, s.disk_total, s.disk_free, care.safe);
    ui.add_space(12.0);
    let mut act = None;
    if widgets::primary(ui, &format!("Open Smart Care  ({} found)", util::size(care.total))).clicked() {
        act = Some(Action::Go(Go::SmartCare));
    }
    if ui.button("Open Storage map").clicked() {
        act = Some(Action::Go(Go::Storage));
    }
    act
}

fn network_tab(ui: &mut Ui, m: &MonState) -> Option<Action> {
    let s = &m.snap;
    ui.label(RichText::new(format!("Down  {}", util::rate(s.rx_rate))).size(18.0).strong().color(TEAL));
    sparkline(ui, &m.rx, None, TEAL, 70.0);
    ui.add_space(6.0);
    ui.label(RichText::new(format!("Up  {}", util::rate(s.tx_rate))).size(18.0).strong().color(Color32::from_rgb(235, 140, 60)));
    sparkline(ui, &m.tx, None, Color32::from_rgb(235, 140, 60), 70.0);
    ui.add_space(8.0);
    if ui.button("Per-app throughput").clicked() {
        return Some(Action::Go(Go::Network));
    }
    None
}

fn battery_tab(ui: &mut Ui, m: &MonState) -> Option<Action> {
    match &m.snap.battery {
        Some(b) => {
            ui.label(RichText::new(format!("{:.0}%", b.percent)).size(30.0).strong().color(theme::GOOD));
            widgets::bar(ui, b.percent / 100.0, String::new(), theme::GOOD);
            if let Some(h) = b.health_percent {
                ui.label(format!("Maximum capacity: {h:.0}% of design"));
            }
            if let Some(c) = b.cycles {
                ui.label(format!("Cycle count: {c}"));
            }
            if let Some(t) = b.temperature_c {
                ui.label(format!("Temperature: {t:.1} °C"));
            }
            if let Some(w) = b.watts {
                ui.label(format!("Power: {w:+.1} W"));
            }
            ui.add_space(8.0);
            if ui.button("Energy impact by app").clicked() {
                return Some(Action::Go(Go::Battery));
            }
        }
        None => {
            ui.label("No battery detected on this Mac.");
        }
    }
    None
}

/// Draws the whole popover. `cats` are the names of the biggest Smart Care categories.
pub fn content(ui: &mut Ui, tab: &mut Tab, m: &MonState, mach: &Machine, care: &CareSummary, cats: &[String]) -> Option<Action> {
    let mut act = None;
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
        icons::tile(ui.painter(), Icon::Health, r.center(), 40.0, theme::ACCENT);
        ui.vertical(|ui| {
            ui.label(RichText::new("MacPulse").size(19.0).strong());
            ui.label(RichText::new(format!("Free · {}", mach.name)).small().color(theme::MUTED));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if round_button(ui, Icon::Startup, theme::BAD) {
                act = Some(Action::Quit);
            }
            if round_button(ui, Icon::AiSettings, Color32::from_rgb(200, 200, 224)) {
                act = Some(Action::Go(Go::Settings));
            }
        });
    });
    ui.add_space(6.0);
    tabs(ui, tab);
    ui.add_space(8.0);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let a = match *tab {
            Tab::Overview => overview(ui, m, care, cats),
            Tab::Memory => memory_tab(ui, m),
            Tab::Disk => disk_tab(ui, m, care),
            Tab::Network => network_tab(ui, m),
            Tab::Battery => battery_tab(ui, m),
        };
        if a.is_some() {
            act = a;
        }
    });
    act
}
