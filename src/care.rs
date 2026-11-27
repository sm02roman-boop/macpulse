use crate::catalog::{Risk, COMMANDS, TARGETS};
use crate::{disk, util};
use eframe::egui::{self, Color32, RichText, Ui};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

enum Msg {
    Scanned(Vec<(usize, u64)>, u64),
    Emptied(u64),
    Cleaned { freed: u64, failed: usize },
    Cmd(String, String),
}

struct Row {
    target: usize,
    bytes: u64,
    selected: bool,
}

pub struct State {
    rows: Vec<Row>,
    ignored: HashSet<String>,
    rx: Option<Receiver<Msg>>,
    busy: bool,
    started: bool,
    status: String,
    confirm: bool,
    cmd_output: Option<(String, String)>,
    trash_bytes: u64,
    confirm_empty: bool,
}

fn ignore_file() -> PathBuf {
    util::home().join(".macpulse").join("ignored.json")
}

fn existing_paths(idx: usize) -> Vec<PathBuf> {
    TARGETS[idx]
        .paths
        .iter()
        .flat_map(|p| util::expand_glob(p))
        .filter(|p| p.is_dir() && util::is_safe_clean_path(p))
        .collect()
}

impl State {
    pub fn new() -> Self {
        let ignored = std::fs::read_to_string(ignore_file())
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_default()
            .into_iter()
            .collect();
        Self {
            rows: vec![],
            ignored,
            rx: None,
            busy: false,
            started: false,
            status: String::new(),
            confirm: false,
            cmd_output: None,
            trash_bytes: 0,
            confirm_empty: false,
        }
    }

    fn save_ignored(&self) {
        let f = ignore_file();
        if let Some(d) = f.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let mut v: Vec<&String> = self.ignored.iter().collect();
        v.sort();
        let _ = std::fs::write(f, serde_json::to_string_pretty(&v).unwrap_or_default());
    }

    fn scan(&mut self, ctx: &egui::Context) {
        self.busy = true;
        self.status = "Scanning…".into();
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let ignored = self.ignored.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut found: Vec<(usize, u64)> = (0..TARGETS.len())
                .into_par_iter()
                .filter(|i| !ignored.contains(TARGETS[*i].id))
                .map(|i| {
                    let b = existing_paths(i).iter().map(|p| disk::size_of(p, None)).sum::<u64>();
                    (i, b)
                })
                .filter(|(_, b)| *b > 0)
                .collect();
            found.sort_by(|a, b| b.1.cmp(&a.1));
            let trash = disk::size_of(&util::home().join(".Trash"), None);
            let _ = tx.send(Msg::Scanned(found, trash));
            ctx.request_repaint();
        });
    }

    fn clean(&mut self, ctx: &egui::Context) {
        self.busy = true;
        self.status = "Moving to Trash…".into();
        let picked: Vec<usize> = self.rows.iter().filter(|r| r.selected).map(|r| r.target).collect();
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let (mut freed, mut failed) = (0u64, 0usize);
            for idx in picked {
                for dir in existing_paths(idx) {
                    let Ok(rd) = std::fs::read_dir(&dir) else { continue };
                    for e in rd.flatten() {
                        let p = e.path();
                        let sz = disk::size_of(&p, None);
                        match util::move_to_trash(&p) {
                            Ok(()) => freed += sz,
                            Err(_) => failed += 1,
                        }
                    }
                }
            }
            let _ = tx.send(Msg::Cleaned { freed, failed });
            ctx.request_repaint();
        });
    }

    fn empty_trash(&mut self, ctx: &egui::Context) {
        self.busy = true;
        self.status = "Emptying Trash…".into();
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let dir = util::home().join(".Trash");
            let mut freed = 0u64;
            if let Ok(rd) = std::fs::read_dir(&dir) {
                for e in rd.flatten() {
                    let p = e.path();
                    let sz = disk::size_of(&p, None);
                    let r = if p.is_dir() && !p.is_symlink() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
                    if r.is_ok() {
                        freed += sz;
                    }
                }
            }
            let _ = tx.send(Msg::Emptied(freed));
            ctx.request_repaint();
        });
    }

    fn run_command(&mut self, ctx: &egui::Context, i: usize) {
        let c = &COMMANDS[i];
        let Some(bin) = c.candidates.iter().find(|p| std::path::Path::new(p).exists()) else {
            self.cmd_output = Some((c.title.into(), "Tool not found on this Mac.".into()));
            return;
        };
        self.busy = true;
        self.status = format!("Running {}…", c.title);
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        let (bin, args, title) = (bin.to_string(), c.args, c.title);
        std::thread::spawn(move || {
            let out = std::process::Command::new(&bin)
                .args(args)
                .env("PATH", "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin")
                .output();
            let text = match out {
                Ok(o) => format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)),
                Err(e) => e.to_string(),
            };
            let _ = tx.send(Msg::Cmd(title.into(), text));
            ctx.request_repaint();
        });
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.rx else { return };
        let Ok(msg) = rx.try_recv() else { return };
        self.rx = None;
        self.busy = false;
        match msg {
            Msg::Scanned(found, trash) => {
                self.trash_bytes = trash;
                self.rows = found
                    .into_iter()
                    .map(|(t, b)| Row { target: t, bytes: b, selected: TARGETS[t].risk == Risk::Safe })
                    .collect();
                self.status = format!("Found {} across {} locations", util::size(self.rows.iter().map(|r| r.bytes).sum()), self.rows.len());
            }
            Msg::Cleaned { freed, failed } => {
                self.status = format!("Moved {} to the Trash{}", util::size(freed), if failed > 0 { format!(" ({failed} items skipped — in use or protected)") } else { String::new() });
                let keep = self.status.clone();
                self.scan(ctx);
                self.status = keep;
            }
            Msg::Emptied(freed) => {
                self.status = format!("Trash emptied: {} freed", util::size(freed));
                let keep = self.status.clone();
                self.scan(ctx);
                self.status = keep;
            }
            Msg::Cmd(title, out) => {
                self.status = format!("{title}: finished");
                self.cmd_output = Some((title, out));
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if !self.started {
            self.started = true;
            self.scan(&ctx);
        }
        self.poll(&ctx);

        let sel_total: u64 = self.rows.iter().filter(|r| r.selected).map(|r| r.bytes).sum();
        ui.horizontal(|ui| {
            ui.heading("Smart Care");
            if self.busy {
                ui.spinner();
            }
            ui.label(&self.status);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = format!("Move {} to Trash…", util::size(sel_total));
                if ui.add_enabled(!self.busy && sel_total > 0, egui::Button::new(label)).clicked() {
                    self.confirm = true;
                }
                if ui.add_enabled(!self.busy, egui::Button::new("↻ Rescan")).clicked() {
                    self.scan(&ctx);
                }
            });
        });
        ui.label("Only the contents of each folder are moved to the Trash, so you can restore anything. “Review” items are never pre-selected.");
        ui.add_space(6.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut toggled_ignore: Option<&'static str> = None;
            let mut last_cat = "";
            for r in self.rows.iter_mut() {
                let t = &TARGETS[r.target];
                if t.category != last_cat {
                    last_cat = t.category;
                    ui.add_space(8.0);
                    ui.label(RichText::new(t.category).strong());
                }
                ui.horizontal(|ui| {
                    ui.checkbox(&mut r.selected, "");
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(t.name);
                            if t.risk == Risk::Review {
                                ui.label(RichText::new("REVIEW").small().color(Color32::from_rgb(235, 170, 60)));
                            }
                        });
                        ui.label(RichText::new(t.note).small().weak());
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(util::size(r.bytes)).strong());
                        if ui.small_button("Ignore").on_hover_text("Never scan or clean this item").clicked() {
                            toggled_ignore = Some(t.id);
                        }
                    });
                });
            }
            if let Some(id) = toggled_ignore {
                self.ignored.insert(id.to_string());
                self.save_ignored();
                self.rows.retain(|r| TARGETS[r.target].id != id);
            }
            if self.rows.is_empty() && !self.busy {
                ui.add_space(20.0);
                ui.label("Nothing to clean right now.");
            }

            ui.add_space(14.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Trash").strong());
                    ui.label(RichText::new("Space from cleaned items is only freed once the Trash is emptied.").small().weak());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_enabled(!self.busy && self.trash_bytes > 0, egui::Button::new(format!("Empty Trash ({})…", util::size(self.trash_bytes)))).clicked() {
                        self.confirm_empty = true;
                    }
                });
            });
            ui.add_space(8.0);
            ui.label(RichText::new("Tool cleanups").strong());
            for (i, c) in COMMANDS.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(c.title);
                        ui.label(RichText::new(c.detail).small().weak());
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(!self.busy, egui::Button::new("Run")).clicked() {
                            self.run_command(&ctx, i);
                        }
                    });
                });
            }
            if !self.ignored.is_empty() {
                ui.add_space(10.0);
                ui.collapsing(format!("Ignore list ({})", self.ignored.len()), |ui| {
                    let mut restore = None;
                    for id in self.ignored.iter() {
                        ui.horizontal(|ui| {
                            let name = TARGETS.iter().find(|t| t.id == id).map(|t| t.name).unwrap_or(id.as_str());
                            ui.label(name);
                            if ui.small_button("Restore").clicked() {
                                restore = Some(id.clone());
                            }
                        });
                    }
                    if let Some(id) = restore {
                        self.ignored.remove(&id);
                        self.save_ignored();
                    }
                });
            }
        });

        if self.confirm {
            egui::Window::new("Move to Trash?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label(format!("{} selected items ({}) will be moved to the Trash.", self.rows.iter().filter(|r| r.selected).count(), util::size(sel_total)));
                    ui.label("Space is only freed after you empty the Trash.");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm = false;
                        }
                        if ui.button("Move to Trash").clicked() {
                            self.confirm = false;
                            self.clean(&ctx);
                        }
                    });
                });
        }
        if self.confirm_empty {
            egui::Window::new("Empty Trash?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label(format!("Permanently delete everything in the Trash ({})?", util::size(self.trash_bytes)));
                    ui.label(RichText::new("This cannot be undone.").color(Color32::from_rgb(232, 90, 79)));
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm_empty = false;
                        }
                        if ui.button("Empty Trash").clicked() {
                            self.confirm_empty = false;
                            self.empty_trash(&ctx);
                        }
                    });
                });
        }
        if let Some((title, out)) = self.cmd_output.clone() {
            let mut open = true;
            egui::Window::new(title).open(&mut open).show(&ctx, |ui| {
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    ui.monospace(if out.trim().is_empty() { "(no output)" } else { out.trim() });
                });
            });
            if !open {
                self.cmd_output = None;
            }
        }
    }
}
