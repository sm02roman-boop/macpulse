//! App uninstaller: trash an app together with the ~/Library data that belongs to its bundle ID.

use crate::leftovers::{bundle_id, walk_apps};
use crate::{disk, util};
use eframe::egui::{self, RichText, Ui};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

#[derive(Clone)]
struct AppEntry {
    name: String,
    path: PathBuf,
    id: String,
    bytes: u64,
}

struct Related {
    path: PathBuf,
    label: &'static str,
    bytes: u64,
    selected: bool,
}

const LOCATIONS: &[(&str, &str)] = &[
    ("Application Support", "Library/Application Support"),
    ("Caches", "Library/Caches"),
    ("Containers", "Library/Containers"),
    ("Group Containers", "Library/Group Containers"),
    ("Preferences", "Library/Preferences"),
    ("Saved State", "Library/Saved Application State"),
    ("HTTP Storage", "Library/HTTPStorages"),
    ("WebKit", "Library/WebKit"),
    ("Logs", "Library/Logs"),
    ("Application Scripts", "Library/Application Scripts"),
];

fn related_for(id: &str) -> Vec<Related> {
    let home = util::home();
    let mut out = vec![];
    for (label, rel) in LOCATIONS {
        let Ok(rd) = std::fs::read_dir(home.join(rel)) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let base = name.strip_suffix(".plist").or_else(|| name.strip_suffix(".savedState")).unwrap_or(&name);
            let hit = base == id
                || base.starts_with(&format!("{id}."))
                || (*label == "Group Containers" && (base == format!("group.{id}") || base.ends_with(&format!(".{id}"))));
            if hit {
                let path = e.path();
                let bytes = disk::size_of(&path, None);
                out.push(Related { path, label, bytes, selected: true });
            }
        }
    }
    out
}

enum Msg {
    Apps(Vec<AppEntry>),
    Done(u64, usize),
}

pub struct State {
    apps: Vec<AppEntry>,
    filter: String,
    selected: Option<usize>,
    related: Vec<Related>,
    rx: Option<Receiver<Msg>>,
    busy: bool,
    started: bool,
    confirm: bool,
    status: String,
}

impl State {
    pub fn new() -> Self {
        Self { apps: vec![], filter: String::new(), selected: None, related: vec![], rx: None, busy: false, started: false, confirm: false, status: String::new() }
    }

    fn load(&mut self, ctx: &egui::Context) {
        self.busy = true;
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut paths = vec![];
            walk_apps(std::path::Path::new("/Applications"), 0, &mut paths);
            walk_apps(&util::home().join("Applications"), 0, &mut paths);
            let mut apps: Vec<AppEntry> = paths
                .par_iter()
                .filter_map(|p| {
                    let id = bundle_id(p)?;
                    if id.starts_with("com.apple.") {
                        return None; // never offer to remove Apple's own apps
                    }
                    Some(AppEntry {
                        name: p.file_stem()?.to_string_lossy().into_owned(),
                        path: p.clone(),
                        id,
                        bytes: disk::size_of(p, None),
                    })
                })
                .collect();
            apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            let _ = tx.send(Msg::Apps(apps));
            ctx.request_repaint();
        });
    }

    fn select(&mut self, i: usize) {
        self.selected = Some(i);
        self.related = related_for(&self.apps[i].id);
    }

    fn uninstall(&mut self, ctx: &egui::Context) {
        let Some(i) = self.selected else { return };
        let app = self.apps[i].clone();
        let picks: Vec<(PathBuf, u64)> = self.related.iter().filter(|r| r.selected).map(|r| (r.path.clone(), r.bytes)).collect();
        self.busy = true;
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let (mut freed, mut failed) = (0u64, 0usize);
            match util::move_to_trash(&app.path) {
                Ok(()) => freed += app.bytes,
                Err(_) => failed += 1,
            }
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
            let _ = tx.send(Msg::Done(freed, failed));
            ctx.request_repaint();
        });
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if !self.started {
            self.started = true;
            self.load(&ctx);
        }
        if let Some(rx) = &self.rx {
            if let Ok(m) = rx.try_recv() {
                self.rx = None;
                self.busy = false;
                match m {
                    Msg::Apps(a) => self.apps = a,
                    Msg::Done(freed, failed) => {
                        self.status = format!("Moved {} to the Trash{}", util::size(freed), if failed > 0 { format!(" ({failed} items skipped)") } else { String::new() });
                        self.selected = None;
                        self.related.clear();
                        self.load(&ctx);
                    }
                }
            }
        }

        ui.horizontal(|ui| {
            if self.busy {
                ui.spinner();
            }
            ui.label(&self.status);
        });
        ui.label("Moves the app and every matching file in ~/Library to the Trash. Apple's own apps are not listed.");
        ui.add_space(6.0);
        let avail = (ui.available_height() - 8.0).max(200.0);
        ui.columns(2, |c| {
            c[0].add(egui::TextEdit::singleline(&mut self.filter).hint_text("Search apps").desired_width(f32::INFINITY));
            let f = self.filter.to_lowercase();
            let mut pick = None;
            egui::ScrollArea::vertical().id_salt("apps").max_height(avail - 30.0).show(&mut c[0], |ui| {
                for (i, a) in self.apps.iter().enumerate() {
                    if !f.is_empty() && !a.name.to_lowercase().contains(&f) {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.selected == Some(i), &a.name).clicked() {
                            pick = Some(i);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(util::size(a.bytes)));
                    });
                }
            });
            if let Some(i) = pick {
                self.select(i);
            }
            let ui = &mut c[1];
            match self.selected.and_then(|i| self.apps.get(i)).cloned() {
                Some(app) => {
                    ui.heading(&app.name);
                    ui.label(RichText::new(&app.id).weak());
                    ui.label(format!("{} · {}", util::size(app.bytes), app.path.display()));
                    ui.separator();
                    ui.label(RichText::new("Related files").strong());
                    egui::ScrollArea::vertical().id_salt("related").max_height(avail - 150.0).show(ui, |ui| {
                        for r in self.related.iter_mut() {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut r.selected, "");
                                ui.vertical(|ui| {
                                    ui.label(r.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
                                    ui.label(RichText::new(r.label).small().weak());
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(util::size(r.bytes)));
                            });
                        }
                        if self.related.is_empty() {
                            ui.label("No leftover files found.");
                        }
                    });
                    let total = app.bytes + self.related.iter().filter(|r| r.selected).map(|r| r.bytes).sum::<u64>();
                    if ui.add_enabled(!self.busy, egui::Button::new(format!("Uninstall — move {} to Trash…", util::size(total)))).clicked() {
                        self.confirm = true;
                    }
                }
                None => {
                    ui.label("Select an app.");
                }
            }
        });
        if self.confirm {
            let name = self.selected.and_then(|i| self.apps.get(i)).map(|a| a.name.clone()).unwrap_or_default();
            egui::Window::new("Uninstall?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label(format!("Move {name} and the selected files to the Trash?"));
                    ui.label("You can restore everything from the Trash.");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm = false;
                        }
                        if ui.button("Uninstall").clicked() {
                            self.confirm = false;
                            self.uninstall(&ctx);
                        }
                    });
                });
        }
    }
}

impl State {
    pub fn ai_context(&self) -> String {
        use std::fmt::Write;
        match self.selected.and_then(|i| self.apps.get(i)) {
            Some(a) => {
                let mut o = format!("Selected app: {} ({}), size {}.\nRelated files in ~/Library:\n", a.name, a.id, util::size(a.bytes));
                for r in &self.related {
                    let _ = writeln!(o, "- {} ({}): {}", r.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), r.label, util::size(r.bytes));
                }
                o
            }
            None => {
                let mut big: Vec<_> = self.apps.iter().collect();
                big.sort_by(|a, b| b.bytes.cmp(&a.bytes));
                let mut o = format!("{} third-party apps installed. Largest:\n", self.apps.len());
                for a in big.iter().take(10) {
                    let _ = writeln!(o, "- {}: {}", a.name, util::size(a.bytes));
                }
                o
            }
        }
    }
}
