use crate::monitor::{MonState, Severity};
use crate::widgets::{bar, card, level_color, ring, sparkline, BLUE, ORANGE, TEAL};
use crate::{mac, treemap, util};
use eframe::egui::{self, Color32, RichText, Ui};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

fn pct(used: u64, total: u64) -> f32 {
    if total == 0 { 0.0 } else { used as f32 / total as f32 * 100.0 }
}

pub fn memory(ui: &mut Ui, m: &MonState, selected: &mut Option<String>) {
    let s = &m.snap;
    ui.horizontal(|ui| {
        let p = pct(s.mem_used, s.mem_total);
        ui.label(RichText::new(format!("{} used of {}", util::bytes(s.mem_used), util::bytes(s.mem_total))).heading());
        ui.label(format!("  {p:.0}% · swap {} · pressure {}", util::bytes(s.swap_used), match s.pressure { 4.. => "critical", 2..=3 => "elevated", _ => "normal" }));
    });
    ui.label("Each block is an app's physical (resident) memory, with helper processes grouped under their parent app.");
    ui.add_space(6.0);
    let items: Vec<treemap::Item> = m
        .groups
        .iter()
        .take(60)
        .map(|g| treemap::Item { id: g.name.clone(), label: g.name.clone(), detail: util::bytes(g.rss), value: g.rss as f64 })
        .collect();
    let h = (ui.available_height() - 6.0).max(240.0);
    ui.columns(2, |c| {
        if let Some(click) = treemap::show(&mut c[0], &items, selected.as_deref(), h) {
            *selected = Some(click.id);
        }
        let ui = &mut c[1];
        match selected.as_ref().and_then(|n| m.groups.iter().find(|g| &g.name == n)) {
            Some(g) => {
                ui.heading(&g.name);
                ui.label(format!("{} across {} processes · {:.0}% CPU", util::bytes(g.rss), g.pids.len(), g.cpu));
                if ui.button(format!("Quit {}", g.name)).on_hover_text("Sends a polite quit request (SIGTERM) to every process in this app").clicked() {
                    mac::terminate(&g.pids);
                }
                ui.separator();
                let mut procs: Vec<_> = s.procs.iter().filter(|p| p.group == g.name).collect();
                procs.sort_by(|a, b| b.rss.cmp(&a.rss));
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for p in procs.iter().take(40) {
                        ui.horizontal(|ui| {
                            ui.label(format!("{} ({})", p.name, p.pid));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(util::bytes(p.rss)));
                        });
                    }
                });
            }
            None => {
                ui.label("Select a block to inspect an app.");
            }
        }
    });
}

pub fn cpu(ui: &mut Ui, m: &MonState) {
    let s = &m.snap;
    egui::ScrollArea::vertical().show(ui, |ui| {
        card(ui, &format!("CPU {:.0}% — rolling 90 s", s.cpu_total), |ui| {
            sparkline(ui, &m.cpu, Some(100.0), BLUE, 90.0);
        });
        ui.add_space(8.0);
        card(ui, "Per-core load", |ui| {
            let layout_known = m.perf_cores + m.eff_cores == s.cores.len();
            for (i, c) in s.cores.iter().enumerate() {
                // On Apple Silicon the efficiency cores are enumerated first.
                let label = if layout_known {
                    if i < m.eff_cores { format!("E{}", i + 1) } else { format!("P{}", i - m.eff_cores + 1) }
                } else {
                    format!("Core {}", i + 1)
                };
                bar(ui, c / 100.0, format!("{label}  {c:.0}%"), level_color(*c));
            }
        });
        ui.add_space(8.0);
        card(ui, "Top apps by CPU", |ui| {
            let mut g: Vec<_> = m.groups.iter().collect();
            g.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
            for g in g.iter().take(12) {
                ui.horizontal(|ui| {
                    ui.label(&g.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(format!("{:.1}%", g.cpu)));
                });
            }
            ui.label(RichText::new("100% = one fully busy core, so multi-threaded apps can exceed 100%.").small().color(crate::theme::MUTED));
        });
    });
}

pub struct NetState {
    rows: Vec<(String, f64, f64)>,
    rx: Option<Receiver<Vec<(String, f64, f64)>>>,
    last: Option<Instant>,
    auto: bool,
    sampled: bool,
}

impl NetState {
    pub fn new() -> Self {
        Self { rows: vec![], rx: None, last: None, auto: true, sampled: false }
    }
}

pub fn network(ui: &mut Ui, m: &MonState, st: &mut NetState) {
    let ctx = ui.ctx().clone();
    if let Some(rx) = &st.rx {
        if let Ok(r) = rx.try_recv() {
            st.rows = r;
            st.rx = None;
            st.sampled = true;
        }
    }
    let due = st.last.map(|t| t.elapsed() > Duration::from_secs(3)).unwrap_or(true);
    if st.rx.is_none() && st.auto && due {
        let (tx, rx) = channel();
        st.rx = Some(rx);
        st.last = Some(Instant::now());
        let c2 = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(mac::nettop());
            c2.request_repaint();
        });
    }
    ctx.request_repaint_after(Duration::from_secs(1));

    card(ui, &format!("Total   Down {}   Up {}", util::rate(m.snap.rx_rate), util::rate(m.snap.tx_rate)), |ui| {
        sparkline(ui, &m.rx, None, TEAL, 50.0);
        sparkline(ui, &m.tx, None, ORANGE, 50.0);
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Per-app throughput (live)").strong());
        ui.checkbox(&mut st.auto, "Auto-refresh");
        if st.rx.is_some() {
            ui.spinner();
        }
    });
    if st.rows.is_empty() {
        ui.label(if st.sampled { "No app is moving data right now." } else if cfg!(target_os = "macos") { "Sampling…" } else { "Per-app network data uses macOS `nettop` and is only available on a Mac." });
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (name, i, o) in st.rows.iter().take(40) {
            ui.horizontal(|ui| {
                ui.label(name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Down {}   Up {}", util::rate(*i), util::rate(*o)));
                });
            });
        }
    });
}

pub struct EnergyState {
    rows: Vec<(String, f32)>,
    rx: Option<Receiver<Vec<(String, f32)>>>,
    last: Option<Instant>,
}

impl EnergyState {
    pub fn new() -> Self {
        Self { rows: vec![], rx: None, last: None }
    }
}

pub fn battery(ui: &mut Ui, m: &MonState, en: &mut EnergyState) {
    let Some(b) = &m.snap.battery else {
        ui.heading("No battery detected");
        ui.label("This Mac has no internal battery, or the battery data is not available on this system.");
        return;
    };
    card(ui, "Battery", |ui| {
        ui.horizontal(|ui| {
            ring(ui, b.percent / 100.0, &format!("{:.0}%", b.percent), Color32::from_rgb(80, 170, 120));
            ui.vertical(|ui| {
                ui.label(if b.charging { "Charging" } else if b.plugged_in { "Plugged in, not charging" } else { "On battery" });
                if let Some(min) = b.minutes_remaining {
                    ui.label(format!("{}h {:02}m remaining", min / 60, min % 60));
                }
                if let Some(w) = b.watts {
                    ui.label(format!("{:+.1} W", w));
                }
            });
        });
    });
    ui.add_space(8.0);
    card(ui, "Health", |ui| {
        if let Some(h) = b.health_percent {
            bar(ui, h / 100.0, format!("Maximum capacity {h:.0}% of design"), if h < 80.0 { ORANGE } else { Color32::from_rgb(80, 170, 120) });
        }
        if let Some(c) = b.cycles { ui.label(format!("Cycle count: {c}")); }
        if let Some(t) = b.temperature_c { ui.label(format!("Temperature: {t:.1} °C")); }
        if let Some(v) = b.voltage_v { ui.label(format!("Voltage: {v:.2} V")); }
    });
    ui.add_space(8.0);
    let ctx = ui.ctx().clone();
    if let Some(rx) = &en.rx {
        if let Ok(r) = rx.try_recv() {
            en.rows = r;
            en.rx = None;
        }
    }
    if en.rx.is_none() && en.last.map(|t| t.elapsed() > Duration::from_secs(6)).unwrap_or(true) {
        let (tx, rx) = channel();
        en.rx = Some(rx);
        en.last = Some(Instant::now());
        std::thread::spawn(move || {
            let _ = tx.send(mac::top_power());
            ctx.request_repaint();
        });
    }
    ui.ctx().request_repaint_after(Duration::from_secs(2));
    card(ui, "Energy impact by process", |ui| {
        if en.rows.is_empty() {
            ui.label(if cfg!(target_os = "macos") { "Measuring…" } else { "Energy impact uses macOS `top` and is only available on a Mac." });
        }
        let max = en.rows.first().map(|r| r.1).unwrap_or(1.0).max(1.0);
        for (name, p) in en.rows.iter().take(10) {
            bar(ui, p / max, format!("{name} — {p:.1}"), ORANGE);
        }
    });
}

pub fn health(ui: &mut Ui, m: &MonState) {
    for f in &m.findings {
        let (icon, color) = match f.severity {
            Severity::Critical => ("•", Color32::from_rgb(232, 90, 79)),
            Severity::Warn => ("•", Color32::from_rgb(235, 170, 60)),
            Severity::Ok => ("•", Color32::from_rgb(80, 170, 120)),
        };
        card(ui, "", |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).color(color).size(30.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new(&f.title).strong());
                    ui.label(&f.detail);
                });
                if let Some(g) = &f.group {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(format!("Quit {g}")).clicked() {
                            if let Some(grp) = m.groups.iter().find(|x| &x.name == g) {
                                mac::terminate(&grp.pids);
                            }
                        }
                    });
                }
            });
        });
        ui.add_space(4.0);
    }
}

impl NetState {
    pub fn ai_context(&self) -> String {
        use std::fmt::Write;
        let mut o = String::from("Apps moving network data right now:\n");
        for (n, i, out) in self.rows.iter().take(10) {
            let _ = writeln!(o, "- {n}: down {}, up {}", util::rate(*i), util::rate(*out));
        }
        if self.rows.is_empty() {
            o.push_str("(no per-app data available)\n");
        }
        o
    }
}

impl EnergyState {
    pub fn ai_context(&self) -> String {
        use std::fmt::Write;
        let mut o = String::from("Energy impact by process (higher = more battery use):\n");
        for (n, p) in self.rows.iter().take(10) {
            let _ = writeln!(o, "- {n}: {p:.1}");
        }
        o
    }
}
