//! Menu bar item with live vitals. macOS only; other platforms get a no-op stub.

use crate::monitor::Snapshot;
use crate::util;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    /// Left click on the menu bar item; rect is the icon's position and size in screen pixels.
    TogglePopover { x: f64, y: f64, w: f64, h: f64 },
    OpenWindow,
    OpenSmartCare,
    Quit,
}

#[cfg(target_os = "macos")]
pub use imp::Tray;
#[cfg(not(target_os = "macos"))]
pub use stub::Tray;

pub fn summary(s: &Snapshot) -> [String; 5] {
    let mem = if s.mem_total > 0 { s.mem_used as f64 / s.mem_total as f64 * 100.0 } else { 0.0 };
    let used = s.disk_total.saturating_sub(s.disk_free);
    [
        format!("CPU  {:.0}%", s.cpu_total),
        format!("Memory  {:.0}%  ({} / {})", mem, util::bytes(s.mem_used), util::bytes(s.mem_total)),
        format!("Disk  {} free of {}", util::size(s.disk_free), util::size(s.disk_total.max(used))),
        format!("Network  ↓ {}  ↑ {}", util::rate(s.rx_rate), util::rate(s.tx_rate)),
        match &s.battery {
            Some(b) => format!("Battery  {:.0}%{}", b.percent, if b.charging { "  (charging)" } else { "" }),
            None => "Battery  none".into(),
        },
    ]
}

#[cfg(not(target_os = "macos"))]
mod stub {
    use super::*;
    pub struct Tray;
    impl Tray {
        pub fn new() -> Option<Self> {
            None
        }
        pub fn update(&self, _s: &Snapshot) {}
        pub fn poll(&self) -> Vec<Action> {
            vec![]
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

    pub struct Tray {
        icon: TrayIcon,
        rows: [MenuItem; 5],
        open: MenuItem,
        care: MenuItem,
        quit: MenuItem,
    }

    /// 36x36 template icon (alpha only): a ring, so macOS tints it for light/dark menu bars.
    fn ring_icon() -> Option<Icon> {
        let n = 36u32;
        let mut px = vec![0u8; (n * n * 4) as usize];
        for y in 0..n {
            for x in 0..n {
                let (dx, dy) = (x as f32 - 17.5, y as f32 - 17.5);
                let d = (dx * dx + dy * dy).sqrt();
                if (10.0..14.0).contains(&d) {
                    let i = ((y * n + x) * 4) as usize;
                    px[i + 3] = 255;
                }
            }
        }
        Icon::from_rgba(px, n, n).ok()
    }

    impl Tray {
        pub fn new() -> Option<Self> {
            let rows = [
                MenuItem::new("CPU", false, None),
                MenuItem::new("Memory", false, None),
                MenuItem::new("Disk", false, None),
                MenuItem::new("Network", false, None),
                MenuItem::new("Battery", false, None),
            ];
            let open = MenuItem::new("Open MacPulse", true, None);
            let care = MenuItem::new("Smart Care…", true, None);
            let quit = MenuItem::new("Quit MacPulse", true, None);
            let menu = Menu::new();
            for r in &rows {
                menu.append(r).ok()?;
            }
            menu.append(&PredefinedMenuItem::separator()).ok()?;
            menu.append(&open).ok()?;
            menu.append(&care).ok()?;
            menu.append(&PredefinedMenuItem::separator()).ok()?;
            menu.append(&quit).ok()?;
            let icon = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .with_icon(ring_icon()?)
                .with_icon_as_template(true)
                .with_title("--%")
                .with_tooltip("MacPulse")
                .build()
                .ok()?;
            Some(Self { icon, rows, open, care, quit })
        }

        pub fn update(&self, s: &Snapshot) {
            self.icon.set_title(Some(format!("{}  CPU {:.0}%", util::bytes(s.mem_used), s.cpu_total)));
            for (item, text) in self.rows.iter().zip(summary(s)) {
                item.set_text(text);
            }
        }

        pub fn poll(&self) -> Vec<Action> {
            let mut out = vec![];
            while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
                if let TrayIconEvent::Click { rect, button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                    out.push(Action::TogglePopover { x: rect.position.x, y: rect.position.y, w: rect.size.width as f64, h: rect.size.height as f64 });
                }
            }
            while let Ok(ev) = MenuEvent::receiver().try_recv() {
                if ev.id == *self.open.id() {
                    out.push(Action::OpenWindow);
                } else if ev.id == *self.care.id() {
                    out.push(Action::OpenSmartCare);
                } else if ev.id == *self.quit.id() {
                    out.push(Action::Quit);
                }
            }
            out
        }
    }
}
