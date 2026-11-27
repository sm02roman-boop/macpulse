//! Startup items audit: LaunchAgents / LaunchDaemons whose program no longer exists.

use crate::util;
use eframe::egui::{self, Color32, RichText, Ui};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Agent {
    pub label: String,
    pub plist: PathBuf,
    pub program: Option<String>,
    pub missing: bool,
    pub user_owned: bool,
}

fn program_of(path: &Path) -> Option<String> {
    let v = plist::Value::from_file(path).ok()?;
    let d = v.as_dictionary()?;
    if let Some(p) = d.get("Program").and_then(|p| p.as_string()) {
        return Some(p.to_string());
    }
    d.get("ProgramArguments")?.as_array()?.first()?.as_string().map(str::to_owned)
}

pub fn scan() -> Vec<Agent> {
    let home = util::home();
    let dirs = [
        (home.join("Library/LaunchAgents"), true),
        (PathBuf::from("/Library/LaunchAgents"), false),
        (PathBuf::from("/Library/LaunchDaemons"), false),
    ];
    let mut out = vec![];
    for (dir, user_owned) in dirs {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x != "plist").unwrap_or(true) {
                continue;
            }
            let program = program_of(&p);
            // Only judge absolute paths; relative ones are resolved via PATH at launch.
            let missing = program.as_ref().map(|x| x.starts_with('/') && !Path::new(x).exists()).unwrap_or(false);
            out.push(Agent {
                label: p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                plist: p,
                program,
                missing,
                user_owned,
            });
        }
    }
    out.sort_by(|a, b| b.missing.cmp(&a.missing).then(a.label.cmp(&b.label)));
    out
}

pub struct State {
    items: Vec<Agent>,
    started: bool,
    status: String,
}

impl State {
    pub fn new() -> Self {
        Self { items: vec![], started: false, status: String::new() }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if !self.started {
            self.started = true;
            self.items = scan();
        }
        let broken = self.items.iter().filter(|a| a.missing).count();
        ui.horizontal(|ui| {
            if ui.button("Rescan").clicked() {
                self.items = scan();
            }
        });
        ui.label(format!("{} launch items, {} pointing at a program that no longer exists. Items in your own LaunchAgents folder can be moved to the Trash; system-wide items are shown read-only.", self.items.len(), broken));
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
        ui.add_space(6.0);
        let mut trash: Option<PathBuf> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for a in &self.items {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(&a.label);
                            if a.missing {
                                ui.label(RichText::new("BROKEN").small().color(Color32::from_rgb(232, 90, 79)));
                            }
                            if !a.user_owned {
                                ui.label(RichText::new("system-wide").small().color(crate::theme::MUTED));
                            }
                        });
                        ui.label(RichText::new(a.program.clone().unwrap_or_else(|| "(no program)".into())).small().color(crate::theme::MUTED));
                    });
                    if a.user_owned && a.missing {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Move to Trash").clicked() {
                                trash = Some(a.plist.clone());
                            }
                        });
                    }
                });
            }
        });
        if let Some(p) = trash {
            self.status = match util::move_to_trash(&p) {
                Ok(()) => format!("Moved {} to the Trash", p.display()),
                Err(e) => e,
            };
            self.items = scan();
        }
    }
}

impl State {
    pub fn ai_context(&self) -> String {
        use std::fmt::Write;
        let broken: Vec<_> = self.items.iter().filter(|a| a.missing).collect();
        let mut o = format!("{} launch agents/daemons; {} point at a program that no longer exists.\n", self.items.len(), broken.len());
        for a in broken.iter().take(15) {
            let _ = writeln!(o, "- BROKEN {} -> {}", a.label, a.program.clone().unwrap_or_default());
        }
        for a in self.items.iter().filter(|a| !a.missing).take(15) {
            let _ = writeln!(o, "- {} ({})", a.label, if a.user_owned { "user" } else { "system-wide" });
        }
        o
    }
}
