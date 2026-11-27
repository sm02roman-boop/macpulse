mod agents;
mod care;
mod catalog;
mod disk;
mod leftovers;
mod mac;
mod monitor;
mod pages;
mod storage;
mod tray;
mod uninstall;
mod treemap;
mod util;
mod widgets;

use eframe::egui;
use std::sync::mpsc::Receiver;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Overview,
    Memory,
    Cpu,
    Storage,
    SmartCare,
    Leftovers,
    Uninstaller,
    Startup,
    Network,
    Battery,
    Health,
}

impl Page {
    const ALL: [Page; 11] = [
        Page::Overview, Page::Memory, Page::Cpu, Page::Storage, Page::SmartCare,
        Page::Leftovers, Page::Uninstaller, Page::Startup, Page::Network, Page::Battery, Page::Health,
    ];
    fn title(self) -> &'static str {
        match self {
            Page::Overview => "Overview",
            Page::Memory => "Memory",
            Page::Cpu => "CPU",
            Page::Storage => "Storage",
            Page::SmartCare => "Smart Care",
            Page::Leftovers => "App Leftovers",
            Page::Uninstaller => "Uninstaller",
            Page::Startup => "Startup Items",
            Page::Network => "Network",
            Page::Battery => "Battery",
            Page::Health => "Health",
        }
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
    tray: Option<tray::Tray>,
    sel_group: Option<String>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
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
            tray: tray::Tray::new(),
            sel_group: None,
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
        egui::SidePanel::left("nav").exact_width(190.0).show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("MacPulse");
            ui.add_space(10.0);
            for p in Page::ALL {
                ui.selectable_value(&mut self.page, p, p.title());
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| match self.page {
            Page::Overview => pages::overview(ui, &self.mon),
            Page::Memory => pages::memory(ui, &self.mon, &mut self.sel_group),
            Page::Cpu => pages::cpu(ui, &self.mon),
            Page::Storage => self.storage.show(ui),
            Page::SmartCare => self.care.show(ui),
            Page::Leftovers => self.left.show(ui),
            Page::Uninstaller => self.uninstall.show(ui),
            Page::Startup => self.agents.show(ui),
            Page::Network => pages::network(ui, &self.mon, &mut self.net),
            Page::Battery => pages::battery(ui, &self.mon, &mut self.energy),
            Page::Health => egui::ScrollArea::vertical().show(ui, |ui| pages::health(ui, &self.mon)).inner,
        });
    }
}

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([900.0, 600.0])
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

    /// Renders every page for several frames on a headless egui context to catch layout panics.
    #[test]
    fn every_page_renders() {
        let ctx = egui::Context::default();
        let mut mon = monitor::MonState::new();
        for i in 0..100 {
            mon.push(snapshot(i));
        }
        assert!(mon.findings.iter().any(|f| f.severity == monitor::Severity::Critical || f.severity == monitor::Severity::Warn));
        let mut st = storage::State::new();
        let mut care = care::State::new();
        let mut left = leftovers::State::new();
        let mut net = pages::NetState::new();
        let mut energy = pages::EnergyState::new();
        let mut un = uninstall::State::new();
        let mut ag = agents::State::new();
        let mut sel = Some("App 3".to_string());
        for frame in 0..6 {
            let mut input = egui::RawInput::default();
            input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 700.0)));
            input.time = Some(frame as f64 * 0.1);
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    pages::overview(ui, &mon);
                    pages::memory(ui, &mon, &mut sel);
                    pages::cpu(ui, &mon);
                    pages::network(ui, &mon, &mut net);
                    pages::battery(ui, &mon, &mut energy);
                    pages::health(ui, &mon);
                    st.show(ui);
                    care.show(ui);
                    left.show(ui);
                    un.show(ui);
                    ag.show(ui);
                });
            });
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
    }
}
