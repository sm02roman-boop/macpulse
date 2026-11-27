mod agents;
mod ai;
mod care;
mod catalog;
mod context;
mod dashboard;
mod disk;
mod icons;
mod leftovers;
mod mac;
mod monitor;
mod pages;
mod storage;
mod theme;
mod tray;
mod treemap;
mod uninstall;
mod util;
mod widgets;

use eframe::egui::{self, vec2, Color32, Rect, RichText, Stroke, StrokeKind};
use icons::Icon;
use std::sync::mpsc::Receiver;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
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
}

impl Page {
    const SECTIONS: [(&'static str, &'static [Page]); 3] = [
        ("MONITOR", &[Page::Overview, Page::Memory, Page::Cpu, Page::Network, Page::Battery, Page::Health]),
        ("CLEAN", &[Page::SmartCare, Page::Storage, Page::Leftovers, Page::Uninstaller, Page::Startup]),
        ("AI", &[Page::Assistant, Page::AiSettings]),
    ];

    fn title(self) -> &'static str {
        match self {
            Page::Overview => "Dashboard",
            Page::Memory => "Memory",
            Page::Cpu => "CPU",
            Page::Network => "Network",
            Page::Battery => "Battery",
            Page::Health => "Health",
            Page::SmartCare => "Smart Care",
            Page::Storage => "Storage",
            Page::Leftovers => "App Leftovers",
            Page::Uninstaller => "Uninstaller",
            Page::Startup => "Startup Items",
            Page::Assistant => "Assistant",
            Page::AiSettings => "AI Settings",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Page::Overview => "Your Mac at a glance",
            Page::Memory => "Where your RAM is going, grouped by app",
            Page::Cpu => "Core load and the busiest apps",
            Page::Network => "Live throughput, per app",
            Page::Battery => "Charge, health and energy use",
            Page::Health => "Automatic checks for common problems",
            Page::SmartCare => "Reclaim space from caches and build junk. Everything goes to the Trash first.",
            Page::Storage => "See exactly what is using your disk",
            Page::Leftovers => "Orphaned files from apps you already deleted",
            Page::Uninstaller => "Remove an app and everything it left behind",
            Page::Startup => "Launch agents and daemons that start with your Mac",
            Page::Assistant => "Ask anything about your Mac",
            Page::AiSettings => "Choose the model that powers AI explanations",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Page::Overview => "Summarize the overall health of this Mac and the single most important thing to do.",
            Page::Memory => "Explain this memory usage. Which apps are using the most and is it a problem?",
            Page::Cpu => "Explain the CPU activity. Is anything unusual? Which apps are responsible?",
            Page::Network => "Explain this network activity. Is any app unexpectedly busy?",
            Page::Battery => "Explain the battery condition and which apps drain it most.",
            Page::Health => "Explain these automatic findings in plain language and say what to do first.",
            Page::SmartCare => "Explain what these cleanup items are, which are safe to clear, and which need care.",
            Page::Storage => "Explain what is using space in this folder and what is reasonable to remove.",
            Page::Leftovers => "Explain what these leftover items are and whether removing them is safe.",
            Page::Uninstaller => "Explain what will be removed if this app is uninstalled.",
            Page::Startup => "Explain these startup items and whether the broken ones matter.",
            Page::Assistant | Page::AiSettings => "",
        }
    }

    fn key(self) -> &'static str {
        self.title()
    }

    fn icon(self) -> Icon {
        match self {
            Page::Overview => Icon::Overview,
            Page::Memory => Icon::Memory,
            Page::Cpu => Icon::Cpu,
            Page::Network => Icon::Network,
            Page::Battery => Icon::Battery,
            Page::Health => Icon::Health,
            Page::SmartCare => Icon::SmartCare,
            Page::Storage => Icon::Storage,
            Page::Leftovers => Icon::Leftovers,
            Page::Uninstaller => Icon::Uninstaller,
            Page::Startup => Icon::Startup,
            Page::Assistant => Icon::Assistant,
            Page::AiSettings => Icon::AiSettings,
        }
    }

    fn tint(self) -> Color32 {
        let (r, g, b) = match self {
            Page::Overview => (50, 120, 230),
            Page::Memory => (150, 100, 235),
            Page::Cpu => (70, 110, 235),
            Page::Network => (40, 170, 190),
            Page::Battery => (52, 170, 110),
            Page::Health => (225, 90, 140),
            Page::SmartCare => (34, 170, 130),
            Page::Storage => (60, 130, 220),
            Page::Leftovers => (235, 140, 60),
            Page::Uninstaller => (225, 80, 80),
            Page::Startup => (70, 180, 110),
            Page::Assistant => (150, 100, 235),
            Page::AiSettings => (110, 120, 150),
        };
        Color32::from_rgb(r, g, b)
    }
}

struct App {
    page: Page,
    mon: monitor::MonState,
    rx: Receiver<monitor::Snapshot>,
    storage: storage::State,
    care: care::State,
    left: leftovers::State,
    net: pages::NetState,
    energy: pages::EnergyState,
    uninstall: uninstall::State,
    agents: agents::State,
    ai: ai::AiHub,
    tray: Option<tray::Tray>,
    sel_group: Option<String>,
    machine: monitor::Machine,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Frosted-glass cards over a solid window (no OS blur layer: it hid the app on macOS).
        theme::set_glass(true);
        theme::apply(&cc.egui_ctx);
        let page = std::env::var("MACPULSE_PAGE").ok().and_then(|k| Page::SECTIONS.iter().flat_map(|(_, p)| p.iter().copied()).find(|p| p.title().eq_ignore_ascii_case(&k))).unwrap_or(Page::Overview);
        Self {
            page,
            mon: monitor::MonState::new(),
            rx: monitor::spawn(cc.egui_ctx.clone()),
            storage: storage::State::new(),
            care: care::State::new(),
            left: leftovers::State::new(),
            net: pages::NetState::new(),
            energy: pages::EnergyState::new(),
            uninstall: uninstall::State::new(),
            agents: agents::State::new(),
            ai: ai::AiHub::new(),
            tray: tray::Tray::new(),
            sel_group: None,
            machine: monitor::Machine::detect(),
        }
    }

    /// Small live label shown at the right of a sidebar row.
    fn badge(&self, p: Page) -> Option<(String, Color32)> {
        let snap = &self.mon.snap;
        let pct = |used: u64, total: u64| if total > 0 { used as f32 / total as f32 * 100.0 } else { 0.0 };
        match p {
            Page::Memory => {
                let v = pct(snap.mem_used, snap.mem_total);
                Some((format!("{v:.0}%"), widgets::level_color(v)))
            }
            Page::Cpu => Some((format!("{:.0}%", snap.cpu_total), widgets::level_color(snap.cpu_total))),
            Page::SmartCare => {
                let (total, _, _) = self.care.summary();
                (total > 0).then(|| (util::size(total), theme::GOOD))
            }
            Page::Storage => (snap.disk_total > 0).then(|| (format!("{} free", util::size(snap.disk_free)), theme::MUTED)),
            Page::Uninstaller => (self.uninstall.app_count() > 0).then(|| (self.uninstall.app_count().to_string(), theme::MUTED)),
            Page::Network => ((snap.rx_rate + snap.tx_rate) > 1000.0).then(|| ("Active".to_string(), theme::MUTED)),
            Page::Battery => snap.battery.as_ref().map(|b| (format!("{:.0}%", b.percent), theme::MUTED)),
            Page::Health => {
                let n = self.mon.findings.iter().filter(|f| f.severity != monitor::Severity::Ok).count();
                (n > 0).then(|| (n.to_string(), theme::WARN))
            }
            _ => None,
        }
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        ui.add_space(f32::from(theme::top_inset()) + 12.0);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), egui::Sense::hover());
            icons::tile(ui.painter(), Icon::Health, r.center(), 32.0, theme::ACCENT);
            ui.label(RichText::new("MacPulse").size(20.0).strong());
        });
        ui.add_space(2.0);
        ui.spacing_mut().item_spacing.y = 3.0;
        for (section, pages) in Page::SECTIONS {
            ui.add_space(8.0);
            ui.label(RichText::new(section).small().strong().color(theme::MUTED));
            for &p in pages {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), egui::Sense::click());
                let selected = self.page == p;
                let badge = self.badge(p);
                let painter = ui.painter();
                if selected {
                    painter.rect_filled(rect, 10.0, theme::ACCENT.gamma_multiply(0.26));
                    painter.rect_stroke(rect, 10.0, Stroke::new(1.0_f32, theme::ACCENT.gamma_multiply(0.6)), StrokeKind::Inside);
                } else if resp.hovered() {
                    painter.rect_filled(rect, 10.0, theme::veil(40));
                }
                icons::tile(painter, p.icon(), rect.left_center() + vec2(21.0, 0.0), 24.0, p.tint());
                let col = if selected { Color32::WHITE } else { Color32::from_rgb(205, 205, 224) };
                painter.text(rect.left_center() + vec2(44.0, 0.0), egui::Align2::LEFT_CENTER, p.title(), egui::FontId::proportional(14.5), col);
                if let Some((text, c)) = badge {
                    let font = egui::FontId::proportional(11.5);
                    let galley = painter.layout_no_wrap(text, font, c);
                    let size = galley.size() + vec2(12.0, 5.0);
                    let br = Rect::from_center_size(rect.right_center() - vec2(10.0 + size.x / 2.0, 0.0), size);
                    painter.rect_filled(br, 8.0, c.gamma_multiply(0.18));
                    painter.galley(br.center() - galley.size() / 2.0, galley, c);
                }
                if resp.clicked() {
                    self.page = p;
                }
            }
        }

        // Footer: reclaimable-space shortcut and machine card.
        ui.add_space((ui.available_height() - 104.0).max(6.0));
        let (total, _, _) = self.care.summary();
        if total > 0 {
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), egui::Sense::click());
            let c = Color32::from_rgb(30, 165, 120);
            ui.painter().rect_filled(rect, 11.0, if resp.hovered() { c.gamma_multiply(1.15) } else { c });
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("{} ready to clean", util::size(total)), egui::FontId::proportional(14.0), Color32::WHITE);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.page = Page::SmartCare;
            }
            ui.add_space(8.0);
        }
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), egui::Sense::hover());
            ui.painter().circle_filled(r.center(), 17.0, theme::ACCENT.gamma_multiply(0.7));
            let initials: String = self.machine.name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect::<String>().to_uppercase();
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, initials, egui::FontId::proportional(13.0), Color32::WHITE);
            ui.vertical(|ui| {
                ui.label(RichText::new(&self.machine.name).strong());
                ui.label(RichText::new(format!("{} · {}", if self.machine.apple_silicon { "Apple Silicon" } else { "Intel Mac" }, self.machine.os)).small().color(theme::MUTED));
            });
        });
    }

    fn draw_page(&mut self, ui: &mut egui::Ui) {
        let page = self.page;
        if page == Page::Overview {
            ui.horizontal(|ui| {
                ui.vertical(|ui| widgets::header(ui, page.title(), page.subtitle()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| dashboard::status_bar(ui, &self.mon));
            });
            let (total, safe, cats) = self.care.summary();
            let summary = dashboard::CareSummary { total, safe, categories: cats };
            let mut go = None;
            egui::ScrollArea::vertical().show(ui, |ui| {
                go = dashboard::show(ui, &self.mon, &self.machine, &summary);
                ui.add_space(10.0);
                if self.ai.card(ui, page.key(), page.hint(), || context::system(&self.mon, 8)) {
                    self.page = Page::AiSettings;
                }
            });
            if self.page != Page::AiSettings {
                match go {
                    Some(dashboard::Go::Memory) => self.page = Page::Memory,
                    Some(dashboard::Go::Cpu) => self.page = Page::Cpu,
                    Some(dashboard::Go::Storage) => self.page = Page::Storage,
                    Some(dashboard::Go::Battery) => self.page = Page::Battery,
                    Some(dashboard::Go::Network) => self.page = Page::Network,
                    Some(dashboard::Go::SmartCare) => self.page = Page::SmartCare,
                    None => {}
                }
            }
            return;
        }
        widgets::header(ui, page.title(), page.subtitle());
        match page {
            Page::Assistant => {
                let ctxt = context::system(&self.mon, 10);
                if self.ai.assistant_page(ui, ctxt) {
                    self.page = Page::AiSettings;
                }
                return;
            }
            Page::AiSettings => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.ai.settings_page(ui);
                });
                return;
            }
            _ => {}
        }
        let wants_settings = self.ai.card(ui, page.key(), page.hint(), || match page {
            Page::Memory => context::system(&self.mon, 12),
            Page::Cpu => context::system(&self.mon, 8),
            Page::Network => format!("{}\n{}", context::system(&self.mon, 4), self.net.ai_context()),
            Page::Battery => format!("{}\n{}", context::system(&self.mon, 4), self.energy.ai_context()),
            Page::SmartCare => self.care.ai_context(),
            Page::Storage => self.storage.ai_context(),
            Page::Leftovers => self.left.ai_context(),
            Page::Uninstaller => self.uninstall.ai_context(),
            Page::Startup => self.agents.ai_context(),
            _ => context::system(&self.mon, 8),
        });
        if wants_settings {
            self.page = Page::AiSettings;
        }
        match page {
            Page::Overview => {}
            Page::Memory => pages::memory(ui, &self.mon, &mut self.sel_group),
            Page::Cpu => pages::cpu(ui, &self.mon),
            Page::Network => pages::network(ui, &self.mon, &mut self.net),
            Page::Battery => pages::battery(ui, &self.mon, &mut self.energy),
            Page::Health => {
                egui::ScrollArea::vertical().show(ui, |ui| pages::health(ui, &self.mon));
            }
            Page::SmartCare => self.care.show(ui),
            Page::Storage => self.storage.show(ui),
            Page::Leftovers => self.left.show(ui),
            Page::Uninstaller => self.uninstall.show(ui),
            Page::Startup => self.agents.show(ui),
            Page::Assistant | Page::AiSettings => {}
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(s) = self.rx.try_recv() {
            if let Some(t) = &self.tray {
                t.update(&s);
            }
            self.mon.push(s);
        }
        if let Some(t) = &self.tray {
            for a in t.poll() {
                match a {
                    tray::Action::OpenWindow => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    tray::Action::OpenSmartCare => {
                        self.page = Page::SmartCare;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    tray::Action::Quit => std::process::exit(0),
                }
            }
        }
        self.care.tick(ctx);
        egui::SidePanel::left("nav")
            .exact_width(220.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(theme::sidebar_fill()).stroke(Stroke::new(1.0_f32, crate::theme::veil(14))).inner_margin(egui::Margin::symmetric(12, 0)))
            .show(ctx, |ui| self.nav(ui));
        let margin = egui::Margin { left: 24, right: 24, top: 18 + theme::top_inset(), bottom: 16 };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::content_fill()).inner_margin(margin))
            .show(ctx, |ui| {
                theme::backdrop(&ui.painter_at(ui.max_rect().expand(40.0)), ui.max_rect());
                self.draw_page(ui)
            });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [11.0 / 255.0, 11.0 / 255.0, 19.0 / 255.0, 1.0]
    }
}

fn main() -> eframe::Result<()> {
    #[allow(unused_mut)]
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1180.0, 780.0])
        .with_min_inner_size([960.0, 720.0])
        .with_title("MacPulse");
    #[cfg(target_os = "macos")]
    {
        // Merge the title bar into the window; the traffic-light buttons float over the sidebar.
        viewport = viewport.with_fullsize_content_view(true).with_title_shown(false).with_titlebar_buttons_shown(true);
    }
    let opts = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("MacPulse", opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}

#[cfg(test)]
mod smoke {
    use super::*;
    use monitor::{ProcInfo, Snapshot};

    fn snapshot(i: u32) -> Snapshot {
        let procs = (0..30)
            .map(|k| ProcInfo {
                pid: k,
                name: format!("proc{k}"),
                group: format!("App {}", k % 7),
                rss: (k as u64 + 1) * 40_000_000,
                cpu: (k % 5) as f32 * 30.0,
            })
            .collect();
        Snapshot {
            cores: vec![10.0, 55.0, 90.0, 3.0],
            cpu_total: 40.0 + i as f32,
            mem_total: 16 << 30,
            mem_used: 9 << 30,
            swap_used: 5 << 30,
            disk_total: 500_000_000_000,
            disk_free: 20_000_000_000,
            rx_rate: 1e6,
            tx_rate: 2e5,
            pressure: 2,
            uptime: 9000,
            disk_name: "Macintosh HD".into(),
            speed_limit: Some(85),
            battery: Some(mac::Battery { percent: 70.0, health_percent: Some(76.0), watts: Some(-8.0), ..Default::default() }),
            procs,
        }
    }

    fn app() -> App {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut mon = monitor::MonState::new();
        for i in 0..100 {
            mon.push(snapshot(i));
        }
        App {
            page: Page::Overview,
            mon,
            rx: std::sync::mpsc::channel().1,
            storage: storage::State::new(),
            care: care::State::new(),
            left: leftovers::State::new(),
            net: pages::NetState::new(),
            energy: pages::EnergyState::new(),
            uninstall: uninstall::State::new(),
            agents: agents::State::new(),
            ai: ai::AiHub::new(),
            tray: None,
            sel_group: Some("App 3".into()),
            machine: monitor::Machine::detect(),
        }
    }

    /// Renders every page (sidebar, header, AI card, content) headlessly to catch layout panics.
    #[test]
    fn every_page_renders() {
        let all: Vec<Page> = Page::SECTIONS.iter().flat_map(|(_, p)| p.iter().copied()).collect();
        assert_eq!(all.len(), 13);
        for glass in [true, false] {
        let ctx = egui::Context::default();
        theme::set_glass(glass);
        theme::apply(&ctx);
        let mut a = app();
        for page in all.iter().copied() {
            a.page = page;
            for frame in 0..3 {
                let mut input = egui::RawInput::default();
                input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1180.0, 780.0)));
                input.time = Some(frame as f64 * 0.1);
                let _ = ctx.run(input, |ctx| {
                    egui::SidePanel::left("nav").show(ctx, |ui| a.nav(ui));
                    egui::CentralPanel::default().show(ctx, |ui| a.draw_page(ui));
                });
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
        }
        }
        theme::set_glass(false);
    }

    #[test]
    fn ai_context_mentions_real_data() {
        let a = app();
        let c = context::system(&a.mon, 5);
        assert!(c.contains("Memory:") && c.contains("App 6") && c.contains("thermally throttled"), "{c}");
    }
}
