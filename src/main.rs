mod agents;
mod ai;
mod care;
mod catalog;
mod context;
mod disk;
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

use eframe::egui::{self, RichText};
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
            Page::Overview => "Overview",
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
            Page::Overview => "Live vitals for your Mac",
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
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        Self {
            page: Page::Overview,
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
        }
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
            ui.painter().rect_filled(r, 9.0, theme::ACCENT);
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "M", egui::FontId::proportional(17.0), egui::Color32::WHITE);
            ui.label(RichText::new("MacPulse").size(20.0).strong());
        });
        ui.add_space(10.0);
        for (section, pages) in Page::SECTIONS {
            ui.add_space(8.0);
            ui.label(RichText::new(section).small().strong().color(theme::MUTED));
            for &p in pages {
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), egui::Sense::click());
                let selected = self.page == p;
                if selected {
                    ui.painter().rect_filled(rect, 9.0, theme::ACCENT.gamma_multiply(0.25));
                    ui.painter().rect_filled(egui::Rect::from_min_size(rect.left_top() + egui::vec2(0.0, 7.0), egui::vec2(3.0, 20.0)), 2.0, theme::ACCENT);
                } else if resp.hovered() {
                    ui.painter().rect_filled(rect, 9.0, egui::Color32::from_white_alpha(10));
                }
                let dot = if p == Page::AiSettings || p == Page::Assistant { theme::ACCENT_SOFT } else { theme::MUTED };
                ui.painter().circle_filled(rect.left_center() + egui::vec2(16.0, 0.0), 3.5, if selected { theme::ACCENT_SOFT } else { dot.gamma_multiply(0.6) });
                ui.painter().text(
                    rect.left_center() + egui::vec2(30.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    p.title(),
                    egui::FontId::proportional(14.5),
                    if selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(205, 205, 225) },
                );
                if resp.clicked() {
                    self.page = p;
                }
            }
        }
    }

    fn draw_page(&mut self, ui: &mut egui::Ui) {
        let page = self.page;
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
                egui::ScrollArea::vertical().show(ui, |ui| self.ai.settings_page(ui));
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
            Page::Overview => pages::overview(ui, &self.mon),
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
        egui::SidePanel::left("nav")
            .exact_width(214.0)
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin::symmetric(12, 0)))
            .show(ctx, |ui| self.nav(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(22, 16)))
            .show(ctx, |ui| self.draw_page(ui));
    }
}

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("MacPulse"),
        ..Default::default()
    };
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
        }
    }

    /// Renders every page (sidebar, header, AI card, content) headlessly to catch layout panics.
    #[test]
    fn every_page_renders() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut a = app();
        let all: Vec<Page> = Page::SECTIONS.iter().flat_map(|(_, p)| p.iter().copied()).collect();
        assert_eq!(all.len(), 13);
        for page in all {
            a.page = page;
            for frame in 0..3 {
                let mut input = egui::RawInput::default();
                input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1180.0, 780.0)));
                input.time = Some(frame as f64 * 0.1);
                let _ = ctx.run(input, |ctx| {
                    egui::SidePanel::left("nav").show(ctx, |ui| a.nav(ui));
                    egui::CentralPanel::default().show(ctx, |ui| a.draw_page(ui));
                });
                std::thread::sleep(std::time::Duration::from_millis(60));
            }
        }
    }

    #[test]
    fn ai_context_mentions_real_data() {
        let a = app();
        let c = context::system(&a.mon, 5);
        assert!(c.contains("Memory:") && c.contains("App 6") && c.contains("thermally throttled"), "{c}");
    }
}
