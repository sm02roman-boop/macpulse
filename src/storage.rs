use crate::{disk, treemap, util};
use eframe::egui::{self, Ui};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

#[derive(Clone)]
pub struct Node {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
}

pub struct State {
    pub cwd: PathBuf,
    stack: Vec<PathBuf>,
    nodes: Vec<Node>,
    rx: Option<Receiver<Node>>,
    cancel: Arc<AtomicBool>,
    scanning: bool,
    selected: Option<PathBuf>,
    confirm: Option<PathBuf>,
    path_text: String,
    status: String,
    started: bool,
}

impl State {
    pub fn new() -> Self {
        let home = util::home();
        Self {
            path_text: home.to_string_lossy().into_owned(),
            cwd: home,
            stack: vec![],
            nodes: vec![],
            rx: None,
            cancel: Arc::new(AtomicBool::new(false)),
            scanning: false,
            selected: None,
            confirm: None,
            status: String::new(),
            started: false,
        }
    }

    fn scan(&mut self, ctx: &egui::Context) {
        self.cancel.store(true, Ordering::Relaxed);
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = cancel.clone();
        self.nodes.clear();
        self.selected = None;
        self.scanning = true;
        self.path_text = self.cwd.to_string_lossy().into_owned();
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let dir = self.cwd.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let Ok(rd) = std::fs::read_dir(&dir) else { return };
            let entries: Vec<_> = rd.flatten().collect();
            entries.par_iter().for_each_with(tx, |tx, e| {
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                let path = e.path();
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let size = disk::size_of(&path, Some(&cancel));
                let _ = tx.send(Node {
                    name: e.file_name().to_string_lossy().into_owned(),
                    path,
                    size,
                    is_dir,
                });
                ctx.request_repaint();
            });
        });
    }

    fn go(&mut self, ctx: &egui::Context, p: PathBuf, push: bool) {
        if push {
            self.stack.push(self.cwd.clone());
        }
        self.cwd = p;
        self.scan(ctx);
    }

    fn poll(&mut self) {
        let mut done = false;
        if let Some(rx) = &self.rx {
            loop {
                match rx.try_recv() {
                    Ok(n) => self.nodes.push(n),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                }
            }
        }
        if done {
            self.scanning = false;
            self.rx = None;
        }
        self.nodes.sort_by(|a, b| b.size.cmp(&a.size));
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if !self.started {
            self.started = true;
            self.scan(&ctx);
        }
        self.poll();

        ui.horizontal(|ui| {
            if ui.add_enabled(!self.stack.is_empty(), egui::Button::new("Back")).clicked() {
                if let Some(p) = self.stack.pop() {
                    self.go(&ctx, p, false);
                }
            }
            if ui.button("Home").clicked() {
                self.go(&ctx, util::home(), true);
            }
            if ui.button("Applications").clicked() {
                self.go(&ctx, PathBuf::from("/Applications"), true);
            }
            if ui.button("Library").clicked() {
                self.go(&ctx, util::home().join("Library"), true);
            }
            let r = ui.add(egui::TextEdit::singleline(&mut self.path_text).desired_width(360.0));
            if (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.button("Go").clicked() {
                let p = PathBuf::from(self.path_text.trim());
                if p.is_dir() {
                    self.go(&ctx, p, true);
                } else {
                    self.status = "Not a folder".into();
                }
            }
            if ui.button("Rescan").clicked() {
                self.scan(&ctx);
            }
            if self.scanning {
                ui.spinner();
            }
        });
        if !self.status.is_empty() {
            ui.colored_label(egui::Color32::LIGHT_RED, &self.status);
        }
        let total: u64 = self.nodes.iter().map(|n| n.size).sum();
        ui.label(format!("{} — {} in {} items", self.cwd.display(), util::size(total), self.nodes.len()));
        ui.add_space(6.0);

        let items: Vec<treemap::Item> = self
            .nodes
            .iter()
            .take(80)
            .map(|n| treemap::Item {
                id: n.path.to_string_lossy().into_owned(),
                label: format!("{}{}", n.name, if n.is_dir { "/" } else { "" }),
                detail: util::size(n.size),
                value: n.size as f64,
            })
            .collect();

        let mut open: Option<PathBuf> = None;
        let mut clicked: Option<treemap::Click> = None;
        let h = (ui.available_height() - 10.0).max(200.0);
        ui.columns(2, |cols| {
            let sel = self.selected.as_ref().map(|p| p.to_string_lossy().into_owned());
            clicked = treemap::show(&mut cols[0], &items, sel.as_deref(), h);
            let ui = &mut cols[1];
            egui::ScrollArea::vertical().max_height(h - 56.0).show(ui, |ui| {
                for n in self.nodes.iter().take(300) {
                    let selected = self.selected.as_ref() == Some(&n.path);
                    ui.horizontal(|ui| {
                        let r = ui.selectable_label(selected, format!("{}{}", n.name, if n.is_dir { "/" } else { "" }));
                        if r.clicked() {
                            self.selected = Some(n.path.clone());
                        }
                        if r.double_clicked() && n.is_dir {
                            open = Some(n.path.clone());
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(util::size(n.size));
                        });
                    });
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                let sel = self.selected.clone();
                if ui.add_enabled(sel.as_ref().map(|p| p.is_dir()).unwrap_or(false), egui::Button::new("Open")).clicked() {
                    open = sel.clone();
                }
                if ui.add_enabled(sel.is_some(), egui::Button::new("Reveal in Finder")).clicked() {
                    if let Some(p) = &sel {
                        let _ = std::process::Command::new("open").arg("-R").arg(p).spawn();
                    }
                }
                if ui.add_enabled(sel.is_some(), egui::Button::new("Move to Trash…")).clicked() {
                    self.confirm = sel;
                }
            });
        });
        if let Some(c) = clicked {
            let p = PathBuf::from(&c.id);
            if c.double && p.is_dir() {
                open = Some(p);
            } else {
                self.selected = Some(p);
            }
        }
        if let Some(p) = open {
            self.go(&ctx, p, true);
        }

        if let Some(p) = self.confirm.clone() {
            egui::Window::new("Move to Trash?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label(p.display().to_string());
                    ui.label("This can be undone from the Trash.");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm = None;
                        }
                        if ui.button("Move to Trash").clicked() {
                            match util::move_to_trash(&p) {
                                Ok(()) => self.status.clear(),
                                Err(e) => self.status = e,
                            }
                            self.confirm = None;
                            self.scan(&ctx);
                        }
                    });
                });
        }
    }
}

impl State {
    pub fn ai_context(&self) -> String {
        use std::fmt::Write;
        let mut o = format!("Folder: {}\nItems (largest first):\n", self.cwd.display());
        for n in self.nodes.iter().take(15) {
            let _ = writeln!(o, "- {}{}: {}", n.name, if n.is_dir { "/" } else { "" }, util::size(n.size));
        }
        o
    }
}
