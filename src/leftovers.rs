use crate::{disk, util};
use eframe::egui::{self, RichText, Ui};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

pub struct Leftover {
    path: PathBuf,
    bundle_id: String,
    location: &'static str,
    bytes: u64,
    selected: bool,
}

enum Msg {
    Found(Vec<Leftover>, usize),
    Trashed(u64, usize),
}

pub struct State {
    items: Vec<Leftover>,
    apps_seen: usize,
    rx: Option<Receiver<Msg>>,
    busy: bool,
    started: bool,
    status: String,
    confirm: bool,
}

pub(crate) fn walk_apps(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "app").unwrap_or(false) {
            out.push(p);
        } else if depth < 3 && e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            walk_apps(&p, depth + 1, out);
        }
    }
}

pub(crate) fn bundle_id(app: &Path) -> Option<String> {
    let v = plist::Value::from_file(app.join("Contents/Info.plist")).ok()?;
    v.as_dictionary()?.get("CFBundleIdentifier")?.as_string().map(str::to_owned)
}

/// Reverse-DNS names only (com.vendor.app). Anything else is too ambiguous to judge.
pub fn looks_like_bundle_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() >= 3
        && s.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
        && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && parts.iter().all(|p| !p.is_empty())
}

fn is_installed(id: &str, installed: &HashSet<String>) -> bool {
    installed.contains(id)
        || installed.iter().any(|i| {
            id.strip_prefix(i.as_str()).map(|r| r.starts_with('.')).unwrap_or(false)
                || i.strip_prefix(id).map(|r| r.starts_with('.')).unwrap_or(false)
        })
}

const SPOTS: &[(&str, &str)] = &[
    ("Containers", "Library/Containers"),
    ("Caches", "Library/Caches"),
    ("Preferences", "Library/Preferences"),
    ("Saved State", "Library/Saved Application State"),
    ("HTTP Storage", "Library/HTTPStorages"),
    ("WebKit Data", "Library/WebKit"),
    ("Logs", "Library/Logs"),
    ("Application Support", "Library/Application Support"),
];

fn find() -> (Vec<Leftover>, usize) {
    let home = util::home();
    let mut apps = vec![];
    for root in ["/Applications", "/System/Applications", "/System/Library/CoreServices"] {
        walk_apps(Path::new(root), 0, &mut apps);
    }
    walk_apps(&home.join("Applications"), 0, &mut apps);
    let installed: HashSet<String> = apps.iter().filter_map(|a| bundle_id(a)).collect();

    let mut out = vec![];
    for (label, rel) in SPOTS {
        let Ok(rd) = std::fs::read_dir(home.join(rel)) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let id = name
                .strip_suffix(".plist")
                .or_else(|| name.strip_suffix(".savedState"))
                .unwrap_or(&name)
                .to_string();
            if !looks_like_bundle_id(&id) || id.starts_with("com.apple.") || is_installed(&id, &installed) {
                continue;
            }
            let path = e.path();
            let bytes = disk::size_of(&path, None);
            out.push(Leftover { path, bundle_id: id, location: label, bytes, selected: false });
        }
    }
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    (out, installed.len())
}

impl State {
    pub fn new() -> Self {
        Self { items: vec![], apps_seen: 0, rx: None, busy: false, started: false, status: String::new(), confirm: false }
    }

    fn scan(&mut self, ctx: &egui::Context) {
        self.busy = true;
        self.status = "Scanning ~/Library…".into();
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let (items, n) = find();
            let _ = tx.send(Msg::Found(items, n));
            ctx.request_repaint();
        });
    }

    fn trash(&mut self, ctx: &egui::Context) {
        self.busy = true;
        let picks: Vec<(PathBuf, u64)> = self.items.iter().filter(|i| i.selected).map(|i| (i.path.clone(), i.bytes)).collect();
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let (mut freed, mut failed) = (0, 0);
            for (p, b) in picks {
                if !util::is_safe_clean_path(&p) {
                    failed += 1;
                    continue;
                }
                match util::move_to_trash(&p) {
                    Ok(()) => freed += b,
                    Err(_) => failed += 1,
                }
            }
            let _ = tx.send(Msg::Trashed(freed, failed));
            ctx.request_repaint();
        });
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if !self.started {
            self.started = true;
            self.scan(&ctx);
        }
        if let Some(rx) = &self.rx {
            if let Ok(m) = rx.try_recv() {
                self.rx = None;
                self.busy = false;
                match m {
                    Msg::Found(items, n) => {
                        self.status = format!("{} candidates ({}) · compared against {n} installed apps", items.len(), util::size(items.iter().map(|i| i.bytes).sum()));
                        self.items = items;
                        self.apps_seen = n;
                    }
                    Msg::Trashed(freed, failed) => {
                        self.status = format!("Moved {} to the Trash{}", util::size(freed), if failed > 0 { format!(" ({failed} skipped)") } else { String::new() });
                        let keep = self.status.clone();
                        self.scan(&ctx);
                        self.status = keep;
                    }
                }
            }
        }

        let sel: u64 = self.items.iter().filter(|i| i.selected).map(|i| i.bytes).sum();
        ui.horizontal(|ui| {
            if self.busy {
                ui.spinner();
            }
            ui.label(&self.status);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(!self.busy && sel > 0, egui::Button::new(format!("Move {} to Trash…", util::size(sel)))).clicked() {
                    self.confirm = true;
                }
                if ui.add_enabled(!self.busy, egui::Button::new("Rescan")).clicked() {
                    self.scan(&ctx);
                }
            });
        });
        ui.label("Files in ~/Library whose app is no longer installed. Matching is by bundle identifier, so nothing is pre-selected: review before trashing. Everything goes to the Trash first.");
        ui.add_space(6.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for it in self.items.iter_mut() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut it.selected, "");
                    ui.vertical(|ui| {
                        ui.label(&it.bundle_id);
                        ui.label(RichText::new(format!("{} · {}", it.location, it.path.display())).small().color(crate::theme::MUTED));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(util::size(it.bytes)).strong());
                    });
                });
            }
            if self.items.is_empty() && !self.busy {
                ui.label("No orphaned app data found.");
            }
        });
        if self.confirm {
            egui::Window::new("Move to Trash?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label(format!("{} items ({}) will be moved to the Trash.", self.items.iter().filter(|i| i.selected).count(), util::size(sel)));
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm = false;
                        }
                        if ui.button("Move to Trash").clicked() {
                            self.confirm = false;
                            self.trash(&ctx);
                        }
                    });
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_id_shape() {
        assert!(looks_like_bundle_id("com.foo.Bar"));
        assert!(!looks_like_bundle_id("Google"));
        assert!(!looks_like_bundle_id("com.foo"));
        assert!(!looks_like_bundle_id("com..foo"));
    }

    #[test]
    fn helper_ids_match_parent() {
        let mut s = HashSet::new();
        s.insert("com.foo.app".to_string());
        assert!(is_installed("com.foo.app.helper", &s));
        assert!(is_installed("com.foo.app", &s));
        assert!(!is_installed("com.other.app", &s));
    }
}

impl State {
    pub fn ai_context(&self) -> String {
        use std::fmt::Write;
        let mut o = format!("{} orphaned items ({}) found in ~/Library for apps that are not installed.\n", self.items.len(), util::size(self.items.iter().map(|i| i.bytes).sum()));
        for i in self.items.iter().take(20) {
            let _ = writeln!(o, "- {} in {}: {}", i.bundle_id, i.location, util::size(i.bytes));
        }
        o
    }
}
