mod agents;
mod ai;
mod appearance;
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

use eframe::egui::{self, pos2, vec2, Color32, Painter, Pos2, Rect, RichText, Stroke, StrokeKind};
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
    glass: bool,
    appearance: appearance::Appearance,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let appearance = appearance::Appearance::load();
        #[allow(unused_mut)]
        let mut glass = appearance.glass;
        #[cfg(target_os = "macos")]
        if glass {
            use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
            // If the blur cannot be applied, fall back to the opaque look instead of a see-through window.
            glass = apply_vibrancy(cc, NSVisualEffectMaterial::HudWindow, None, Some(12.0)).is_ok();
        }
        #[cfg(not(target_os = "macos"))]
        {
            glass = false;
        }
        theme::set_glass(glass);
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
            glass,
            appearance,
        }
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        ui.add_space(f32::from(theme::top_inset()) + 12.0);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), egui::Sense::hover());
            ui.painter().rect_filled(r, 10.0, theme::ACCENT);
            ui.painter().rect_filled(Rect::from_min_size(r.min + vec2(2.0, 2.0), vec2(28.0, 6.0)), 4.0, Color32::from_white_alpha(36));
            draw_icon(ui.painter(), Page::Health, r.center(), Color32::WHITE);
            ui.label(RichText::new("MacPulse").size(20.0).strong());
        });
        ui.add_space(8.0);
        for (section, pages) in Page::SECTIONS {
            ui.add_space(10.0);
            ui.label(RichText::new(section).small().strong().color(theme::MUTED));
            for &p in pages {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), egui::Sense::click());
                let selected = self.page == p;
                let painter = ui.painter();
                if selected {
                    painter.rect_filled(rect, 10.0, theme::ACCENT.gamma_multiply(0.26));
                    painter.rect_stroke(rect, 10.0, Stroke::new(1.0, theme::ACCENT.gamma_multiply(0.6)), StrokeKind::Inside);
                } else if resp.hovered() {
                    painter.rect_filled(rect, 10.0, Color32::from_white_alpha(12));
                }
                let col = if selected { Color32::WHITE } else { Color32::from_rgb(190, 190, 214) };
                draw_icon(painter, p, rect.left_center() + vec2(20.0, 0.0), if selected { theme::ACCENT_SOFT } else { col });
                painter.text(rect.left_center() + vec2(40.0, 0.0), egui::Align2::LEFT_CENTER, p.title(), egui::FontId::proportional(14.5), col);
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
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.ai.settings_page(ui);
                    ui.add_space(8.0);
                    widgets::card(ui, "Appearance", |ui| {
                        let mut on = self.appearance.glass;
                        if ui.checkbox(&mut on, "Glass window (macOS blur behind the app)").changed() {
                            self.appearance.glass = on;
                            self.appearance.save();
                        }
                        ui.label(RichText::new("Takes effect after you restart MacPulse. Turn it off for a solid dark window.").color(theme::MUTED));
                    });
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

/// Small vector icons drawn with the painter (no font or image dependencies).
fn draw_icon(p: &Painter, page: Page, c: Pos2, col: Color32) {
    let st = Stroke::new(1.6, col);
    let line = |a: Pos2, b: Pos2| p.line_segment([c + a.to_vec2(), c + b.to_vec2()], st);
    let rect = |w: f32, h: f32, r: f32| p.rect_stroke(Rect::from_center_size(c, vec2(w, h)), r, st, StrokeKind::Middle);
    match page {
        Page::Overview => {
            for (dx, dy) in [(-4.5, -4.5), (4.5, -4.5), (-4.5, 4.5), (4.5, 4.5)] {
                p.rect_stroke(Rect::from_center_size(c + vec2(dx, dy), vec2(6.0, 6.0)), 1.5, st, StrokeKind::Middle);
            }
        }
        Page::Memory => {
            rect(14.0, 8.0, 2.0);
            for x in [-4.0, 0.0, 4.0] {
                line(pos2(x, -4.0), pos2(x, -7.0));
                line(pos2(x, 4.0), pos2(x, 7.0));
            }
        }
        Page::Cpu => {
            rect(11.0, 11.0, 2.5);
            p.rect_filled(Rect::from_center_size(c, vec2(4.0, 4.0)), 1.0, col);
            for v in [-3.0, 3.0] {
                line(pos2(v, -5.5), pos2(v, -8.0));
                line(pos2(v, 5.5), pos2(v, 8.0));
                line(pos2(-5.5, v), pos2(-8.0, v));
                line(pos2(5.5, v), pos2(8.0, v));
            }
        }
        Page::Network => {
            line(pos2(-4.0, 7.0), pos2(-4.0, -6.0));
            line(pos2(-4.0, -7.0), pos2(-7.0, -3.5));
            line(pos2(-4.0, -7.0), pos2(-1.0, -3.5));
            line(pos2(4.0, -7.0), pos2(4.0, 6.0));
            line(pos2(4.0, 7.0), pos2(1.0, 3.5));
            line(pos2(4.0, 7.0), pos2(7.0, 3.5));
        }
        Page::Battery => {
            p.rect_stroke(Rect::from_center_size(c + vec2(-1.0, 0.0), vec2(14.0, 8.5)), 2.0, st, StrokeKind::Middle);
            p.rect_filled(Rect::from_center_size(c + vec2(7.5, 0.0), vec2(2.0, 4.0)), 1.0, col);
            p.rect_filled(Rect::from_center_size(c + vec2(-3.0, 0.0), vec2(7.0, 4.5)), 1.0, col);
        }
        Page::Health => {
            let pts = [(-8.0, 0.5), (-4.0, 0.5), (-2.0, -6.0), (1.0, 6.0), (3.0, 0.5), (8.0, 0.5)];
            for w in pts.windows(2) {
                line(pos2(w[0].0, w[0].1), pos2(w[1].0, w[1].1));
            }
        }
        Page::SmartCare => {
            let pts: Vec<Pos2> = [(0.0, -8.0), (2.0, -2.0), (8.0, 0.0), (2.0, 2.0), (0.0, 8.0), (-2.0, 2.0), (-8.0, 0.0), (-2.0, -2.0)]
                .iter()
                .map(|(x, y)| c + vec2(*x, *y))
                .collect();
            p.add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
        }
        Page::Storage => {
            p.circle_stroke(c, 7.5, st);
            p.circle_stroke(c, 2.5, st);
            line(pos2(2.5, -2.5), pos2(6.0, -6.0));
        }
        Page::Leftovers => {
            p.rect_stroke(Rect::from_center_size(c + vec2(0.0, 1.5), vec2(11.0, 11.0)), 2.0, st, StrokeKind::Middle);
            line(pos2(-7.0, -5.0), pos2(7.0, -5.0));
            line(pos2(-2.5, -5.0), pos2(-2.5, -7.5));
            line(pos2(2.5, -5.0), pos2(2.5, -7.5));
            line(pos2(-2.5, -1.0), pos2(-2.5, 5.0));
            line(pos2(2.5, -1.0), pos2(2.5, 5.0));
        }
        Page::Uninstaller => {
            rect(13.0, 13.0, 3.0);
            line(pos2(-3.5, -3.5), pos2(3.5, 3.5));
            line(pos2(3.5, -3.5), pos2(-3.5, 3.5));
        }
        Page::Startup => {
            let pts: Vec<Pos2> = (0..=36)
                .map(|i| {
                    let a = (-60.0_f32 + 300.0 * i as f32 / 36.0 - 90.0).to_radians();
                    c + vec2(a.cos(), a.sin()) * 7.0
                })
                .collect();
            p.add(egui::Shape::line(pts, st));
            line(pos2(0.0, -8.5), pos2(0.0, -1.0));
        }
        Page::Assistant => {
            p.rect_stroke(Rect::from_center_size(c + vec2(0.0, -1.0), vec2(15.0, 10.5)), 3.0, st, StrokeKind::Middle);
            line(pos2(-3.0, 4.5), pos2(-5.5, 8.0));
            line(pos2(-3.0, 4.5), pos2(1.0, 4.5));
        }
        Page::AiSettings => {
            for (y, x) in [(-5.0, -3.0), (0.0, 3.0), (5.0, -1.0)] {
                line(pos2(-8.0, y), pos2(8.0, y));
                p.circle_filled(c + vec2(x, y), 2.4, col);
            }
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
            .exact_width(220.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(theme::sidebar_fill()).stroke(Stroke::new(1.0, Color32::from_white_alpha(if self.glass { 14 } else { 0 }))).inner_margin(egui::Margin::symmetric(12, 0)))
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
        if self.glass { [0.0, 0.0, 0.0, 0.0] } else { [15.0 / 255.0, 15.0 / 255.0, 22.0 / 255.0, 1.0] }
    }
}

fn main() -> eframe::Result<()> {
    let glass = cfg!(target_os = "macos") && appearance::Appearance::load().glass;
    #[allow(unused_mut)]
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1180.0, 780.0])
        .with_min_inner_size([960.0, 640.0])
        .with_title("MacPulse")
        .with_transparent(glass);
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
            glass: true,
            appearance: appearance::Appearance::default(),
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
        a.glass = glass;
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
