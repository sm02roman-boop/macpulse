//! AI explanations. Talks to any OpenAI-compatible chat endpoint. The default is a local
//! Ollama server on this Mac, so it is free and nothing leaves the machine.

use crate::{theme, util};
use eframe::egui::{self, Color32, RichText, Stroke, Ui};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

pub const SYSTEM_PROMPT: &str = "You are the built-in assistant of MacPulse, a Mac system monitor and cleaner. \
You receive diagnostic data from the user's own Mac. Explain in plain, calm language what it means: \
what is normal, what looks wrong, and why. Then give at most 3 concrete next steps using MacPulse's own \
features (Smart Care, Uninstaller, App Leftovers, Startup Items, the Quit button, Storage). \
Never suggest commands that permanently delete data, disable macOS security, or use sudo. \
Only use facts present in the data; if something is not shown, say you cannot tell. \
Keep the answer under 150 words, no headings, short bullet points with a dash.";

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    /// "openai" (Ollama, LM Studio, any OpenAI-compatible server) or "anthropic" (Claude).
    pub kind: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self { base_url: "http://localhost:11434/v1".into(), model: "llama3.2:3b".into(), api_key: String::new(), kind: "openai".into() }
    }
}

pub const CLAUDE_URL: &str = "https://api.anthropic.com";
pub const CLAUDE_MODEL: &str = "claude-haiku-5-5";

impl Settings {
    fn path() -> PathBuf {
        util::home().join(".macpulse").join("settings.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if std::fs::write(&p, serde_json::to_string_pretty(self).unwrap_or_default()).is_ok() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
            }
        }
    }

    pub fn is_claude(&self) -> bool {
        self.kind == "anthropic" || self.base_url.contains("api.anthropic.com")
    }

    pub fn use_ollama(&mut self) {
        *self = Self { api_key: String::new(), ..Self::default() };
    }

    pub fn use_claude(&mut self) {
        self.kind = "anthropic".into();
        self.base_url = CLAUDE_URL.into();
        self.model = CLAUDE_MODEL.into();
    }

    pub fn is_local(&self) -> bool {
        let u = self.base_url.to_lowercase();
        u.contains("://localhost") || u.contains("://127.0.0.1") || u.contains("://[::1]")
    }

    pub fn host(&self) -> String {
        let u = self.base_url.split("://").nth(1).unwrap_or(&self.base_url);
        u.split('/').next().unwrap_or(u).to_string()
    }
}

pub type Msg = (String, String);

/// Strip the markdown noise small models like to add.
pub fn clean(text: &str) -> String {
    text.lines()
        .map(|l| {
            let l = l.trim_start_matches('#').trim_start();
            let l = if let Some(r) = l.strip_prefix("* ") { format!("- {r}") } else { l.to_string() };
            l.replace("**", "").replace("__", "")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Pull the human-readable message out of an API error body (works for Claude and OpenAI-style).
fn api_message(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_owned))
        .unwrap_or_else(|| body.chars().take(160).collect())
}

fn describe_status(code: u16, body: &str, s: &Settings) -> String {
    let short = api_message(body);
    match code {
        401 | 403 if s.is_claude() => "Claude rejected the API key. Create a key at console.anthropic.com and paste it in AI Settings.".into(),
        400 if s.is_claude() && short.to_lowercase().contains("credit") => format!("Your Anthropic account has no credit left: {short}"),
        404 if body.to_lowercase().contains("model") => {
            format!("Model \"{}\" is not installed. In Terminal run:  ollama pull {}", s.model, s.model)
        }
        401 | 403 => "The server rejected the API key. Check it in AI Settings.".into(),
        _ => format!("The AI server answered with error {code}. {short}"),
    }
}

pub fn chat(s: &Settings, messages: &[Msg]) -> Result<String, String> {
    if s.is_claude() {
        return chat_claude(s, messages);
    }
    let url = format!("{}/chat/completions", s.base_url.trim_end_matches('/'));
    let msgs: Vec<Value> = messages.iter().map(|(r, c)| json!({"role": r, "content": c})).collect();
    let mut req = ureq::post(&url).timeout(Duration::from_secs(180));
    if !s.api_key.trim().is_empty() {
        req = req.set("Authorization", &format!("Bearer {}", s.api_key.trim()));
    }
    let body = json!({"model": s.model, "messages": msgs, "stream": false, "temperature": 0.3, "max_tokens": 500});
    match req.send_json(body) {
        Ok(r) => {
            let v: Value = r.into_json().map_err(|e| e.to_string())?;
            v["choices"][0]["message"]["content"]
                .as_str()
                .map(clean)
                .filter(|t| !t.is_empty())
                .ok_or_else(|| "The model returned an empty answer. Try again.".to_string())
        }
        Err(ureq::Error::Status(code, r)) => Err(describe_status(code, &r.into_string().unwrap_or_default(), s)),
        Err(ureq::Error::Transport(t)) => {
            Err(format!("Can't reach {} ({t}). If you use Ollama, make sure it is installed and running.", s.host()))
        }
    }
}

/// Claude's Messages API: system prompt is separate and turns must alternate.
fn chat_claude(s: &Settings, messages: &[Msg]) -> Result<String, String> {
    if s.api_key.trim().is_empty() {
        return Err("Add your Anthropic API key in AI Settings to use Claude.".into());
    }
    let system: String = messages.iter().filter(|(r, _)| r == "system").map(|(_, c)| c.as_str()).collect::<Vec<_>>().join("\n");
    let mut turns: Vec<Value> = vec![];
    for (role, text) in messages.iter().filter(|(r, _)| r != "system") {
        match turns.last_mut() {
            Some(last) if last["role"] == role.as_str() => {
                let merged = format!("{}\n\n{}", last["content"].as_str().unwrap_or(""), text);
                last["content"] = Value::String(merged);
            }
            _ => turns.push(json!({"role": role, "content": text})),
        }
    }
    let url = format!("{}/v1/messages", s.base_url.trim_end_matches('/'));
    let body = json!({"model": s.model, "max_tokens": 600, "system": system, "messages": turns});
    let req = ureq::post(&url)
        .timeout(Duration::from_secs(120))
        .set("x-api-key", s.api_key.trim())
        .set("anthropic-version", "2023-06-01");
    match req.send_json(body) {
        Ok(r) => {
            let v: Value = r.into_json().map_err(|e| e.to_string())?;
            let text: String = v["content"]
                .as_array()
                .map(|a| a.iter().filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("\n"))
                .unwrap_or_default();
            let text = clean(&text);
            if text.is_empty() { Err("Claude returned an empty answer. Try again.".into()) } else { Ok(text) }
        }
        Err(ureq::Error::Status(code, r)) => Err(describe_status(code, &r.into_string().unwrap_or_default(), s)),
        Err(ureq::Error::Transport(t)) => Err(format!("Can't reach {} ({t}). Check your internet connection.", s.host())),
    }
}

pub fn list_models(s: &Settings) -> Result<Vec<String>, String> {
    if s.is_claude() {
        if s.api_key.trim().is_empty() {
            return Err("Paste your Anthropic API key, then click Save & test.".into());
        }
        let url = format!("{}/v1/models?limit=50", s.base_url.trim_end_matches('/'));
        let req = ureq::get(&url).timeout(Duration::from_secs(10)).set("x-api-key", s.api_key.trim()).set("anthropic-version", "2023-06-01");
        return match req.call() {
            Ok(r) => {
                let v: Value = r.into_json().map_err(|e| e.to_string())?;
                let mut out: Vec<String> = v["data"].as_array().map(|a| a.iter().filter_map(|m| m["id"].as_str().map(str::to_owned)).collect()).unwrap_or_default();
                out.sort();
                Ok(out)
            }
            Err(ureq::Error::Status(code, r)) => Err(describe_status(code, &r.into_string().unwrap_or_default(), s)),
            Err(ureq::Error::Transport(t)) => Err(format!("Can't reach {} ({t})", s.host())),
        };
    }
    let url = format!("{}/models", s.base_url.trim_end_matches('/'));
    let mut req = ureq::get(&url).timeout(Duration::from_secs(8));
    if !s.api_key.trim().is_empty() {
        req = req.set("Authorization", &format!("Bearer {}", s.api_key.trim()));
    }
    match req.call() {
        Ok(r) => {
            let v: Value = r.into_json().map_err(|e| e.to_string())?;
            let mut out: Vec<String> =
                v["data"].as_array().map(|a| a.iter().filter_map(|m| m["id"].as_str().map(str::to_owned)).collect()).unwrap_or_default();
            out.sort();
            Ok(out)
        }
        Err(ureq::Error::Status(code, r)) => Err(describe_status(code, &r.into_string().unwrap_or_default(), s)),
        Err(ureq::Error::Transport(t)) => Err(format!("Can't reach {} ({t})", s.host())),
    }
}

type Reply = Result<String, String>;

fn spawn_chat(s: Settings, ctx: &egui::Context, msgs: Vec<Msg>) -> Receiver<Reply> {
    let (tx, rx) = channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(chat(&s, &msgs));
        ctx.request_repaint();
    });
    rx
}

#[derive(Default)]
struct Panel {
    text: Option<String>,
    error: Option<String>,
    rx: Option<Receiver<Reply>>,
}

#[derive(Default)]
struct ChatState {
    log: Vec<Msg>,
    input: String,
    rx: Option<Receiver<Reply>>,
    error: Option<String>,
}

pub struct AiHub {
    pub settings: Settings,
    panels: HashMap<&'static str, Panel>,
    chat: ChatState,
    models: Vec<String>,
    models_rx: Option<Receiver<Result<Vec<String>, String>>>,
    status: Option<Result<String, String>>,
    fetched: bool,
}

fn pill(ui: &mut Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.22))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).small().strong().color(color));
        });
}

fn ai_badge(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 11.0, theme::ACCENT);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "AI", egui::FontId::proportional(16.0), Color32::WHITE);
}

impl AiHub {
    pub fn new() -> Self {
        Self {
            settings: Settings::load(),
            panels: HashMap::new(),
            chat: ChatState::default(),
            models: vec![],
            models_rx: None,
            status: None,
            fetched: false,
        }
    }

    fn privacy_pill(&self, ui: &mut Ui) {
        if self.settings.is_local() {
            pill(ui, "FREE · ON THIS MAC", theme::GOOD);
        } else {
            pill(ui, &format!("SENDS DATA TO {}", self.settings.host().to_uppercase()), theme::WARN);
        }
    }

    /// The "Explain with AI" card shown at the top of every page. `context` is only built when
    /// the user clicks the button. Returns true if the user asked to open AI settings.
    pub fn card(&mut self, ui: &mut Ui, key: &'static str, hint: &str, context: impl FnOnce() -> String) -> bool {
        let ctx = ui.ctx().clone();
        let settings = self.settings.clone();
        let local = settings.is_local();
        let host = settings.host();
        let panel = self.panels.entry(key).or_default();
        if let Some(rx) = &panel.rx {
            if let Ok(r) = rx.try_recv() {
                panel.rx = None;
                match r {
                    Ok(t) => {
                        panel.text = Some(t);
                        panel.error = None;
                    }
                    Err(e) => panel.error = Some(e),
                }
            }
        }
        let busy = panel.rx.is_some();
        let (mut start, mut open_settings) = (false, false);
        egui::Frame::new()
            .fill(Color32::from_rgb(38, 30, 62))
            .stroke(Stroke::new(1.0, theme::ACCENT.gamma_multiply(0.6)))
            .corner_radius(14)
            .inner_margin(14)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ai_badge(ui);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Explain this page with AI").size(17.0).strong());
                            if local {
                                pill(ui, "FREE · ON THIS MAC", theme::GOOD);
                            } else {
                                pill(ui, &format!("SENDS DATA TO {}", host.to_uppercase()), theme::WARN);
                            }
                        });
                        ui.label(RichText::new("A short, plain-language explanation of what you see here.").color(theme::MUTED));
                    });
                });
                ui.horizontal(|ui| {
                    if busy {
                        ui.spinner();
                        ui.label("Thinking…");
                    } else if ui.add(egui::Button::new(RichText::new("Explain with AI  →").strong()).fill(theme::ACCENT)).clicked() {
                        start = true;
                    }
                    if panel.text.is_some() && !busy {
                        if ui.button("Copy").clicked() {
                            ui.ctx().copy_text(panel.text.clone().unwrap_or_default());
                        }
                        if ui.button("Clear").clicked() {
                            panel.text = None;
                        }
                    }
                });
                if let Some(t) = &panel.text {
                    ui.separator();
                    ui.label(t);
                }
                if let Some(e) = &panel.error {
                    ui.separator();
                    ui.label(RichText::new(e).color(theme::BAD));
                    if ui.link("Open AI settings").clicked() {
                        open_settings = true;
                    }
                }
            });
        if start {
            let panel = self.panels.entry(key).or_default();
            panel.error = None;
            let prompt = format!("{hint}\n\n{}", context());
            panel.rx = Some(spawn_chat(settings, &ctx, vec![("system".into(), SYSTEM_PROMPT.into()), ("user".into(), prompt)]));
        }
        ui.add_space(6.0);
        open_settings
    }

    fn fetch_models(&mut self, ctx: &egui::Context) {
        let s = self.settings.clone();
        let (tx, rx) = channel();
        self.models_rx = Some(rx);
        self.status = None;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(list_models(&s));
            ctx.request_repaint();
        });
    }

    pub fn settings_page(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if !self.fetched {
            self.fetched = true;
            self.fetch_models(&ctx);
        }
        if let Some(rx) = &self.models_rx {
            if let Ok(r) = rx.try_recv() {
                self.models_rx = None;
                match r {
                    Ok(m) => {
                        self.status = Some(Ok(format!("Connected. {} model(s) available.", m.len())));
                        self.models = m;
                    }
                    Err(e) => self.status = Some(Err(e)),
                }
            }
        }

        crate::widgets::card(ui, "Connection", |ui| {
            ui.horizontal(|ui| {
                ui.label("Presets");
                if ui.button("Ollama (local, free)").clicked() {
                    self.settings.use_ollama();
                }
                if ui.button("LM Studio (local, free)").clicked() {
                    self.settings.use_ollama();
                    self.settings.base_url = "http://localhost:1234/v1".into();
                }
                if ui.button("Claude (Anthropic API)").clicked() {
                    self.settings.use_claude();
                    self.models.clear();
                    self.status = None;
                }
            });
            egui::Grid::new("ai-settings").num_columns(2).spacing([12.0, 10.0]).show(ui, |ui| {
                ui.label("Server URL");
                ui.add(egui::TextEdit::singleline(&mut self.settings.base_url).desired_width(380.0));
                ui.end_row();
                ui.label("Model");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.settings.model).desired_width(220.0));
                    if !self.models.is_empty() {
                        egui::ComboBox::from_id_salt("models").selected_text("Installed…").show_ui(ui, |ui| {
                            for m in self.models.clone() {
                                if ui.selectable_label(self.settings.model == m, &m).clicked() {
                                    self.settings.model = m;
                                }
                            }
                        });
                    }
                });
                ui.end_row();
                ui.label("API key");
                let hint = if self.settings.is_claude() { "sk-ant-…  (required for Claude)" } else { "not needed for local servers" };
                ui.add(egui::TextEdit::singleline(&mut self.settings.api_key).password(true).hint_text(hint).desired_width(380.0));
                ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("Save & test").strong()).fill(theme::ACCENT)).clicked() {
                    self.settings.save();
                    self.fetch_models(&ctx);
                }
                if self.models_rx.is_some() {
                    ui.spinner();
                }
                self.privacy_pill(ui);
            });
            match &self.status {
                Some(Ok(t)) => ui.label(RichText::new(t).color(theme::GOOD)),
                Some(Err(e)) => ui.label(RichText::new(e).color(theme::BAD)),
                None => ui.label(""),
            };
            if !self.settings.is_local() {
                ui.label(RichText::new("This server is not on your Mac. When you click an Explain button, the page's diagnostic data (app names, file names, sizes) is sent to it.").color(theme::WARN));
            }
        });
        ui.add_space(8.0);
        crate::widgets::card(ui, "Use Claude", |ui| {
            ui.label("1.  Go to  console.anthropic.com  and sign in, then open API keys and create a key.");
            ui.label("2.  Click the Claude preset above, paste the key, and press Save & test.");
            ui.label(RichText::new("A claude.ai subscription does not include API access. The API is billed separately by usage, and the Haiku models are the cheapest. When you click Explain, the page's app names and file names are sent to Anthropic.").color(theme::MUTED));
        });
        ui.add_space(8.0);
        crate::widgets::card(ui, "Free setup with Ollama (about 3 minutes)", |ui| {
            ui.label("1.  Install Ollama from  ollama.com/download  and open it once.");
            ui.label("2.  In Terminal, download a small model (about 2 GB):");
            ui.monospace("ollama pull llama3.2:3b");
            ui.label("3.  Come back here and click Save & test.");
            ui.label(RichText::new("The model runs entirely on your Mac. Nothing is sent anywhere. Older or Intel Macs work but answer more slowly.").color(theme::MUTED));
        });
        ui.add_space(8.0);
        crate::widgets::card(ui, "Privacy", |ui| {
            ui.label("AI only runs when you click a button. Settings are stored in ~/.macpulse/settings.json (readable only by you).");
        });
    }

    pub fn assistant_page(&mut self, ui: &mut Ui, context: String) -> bool {
        let ctx = ui.ctx().clone();
        if let Some(rx) = &self.chat.rx {
            if let Ok(r) = rx.try_recv() {
                self.chat.rx = None;
                match r {
                    Ok(t) => self.chat.log.push(("assistant".into(), t)),
                    Err(e) => self.chat.error = Some(e),
                }
            }
        }
        let mut open_settings = false;
        let mut send: Option<String> = None;
        let busy = self.chat.rx.is_some();

        ui.horizontal(|ui| {
            self.privacy_pill(ui);
            ui.label(RichText::new(format!("Model: {}", self.settings.model)).color(theme::MUTED));
            if ui.link("AI settings").clicked() {
                open_settings = true;
            }
        });
        if self.chat.log.is_empty() {
            ui.label(RichText::new("Try one of these:").color(theme::MUTED));
            ui.horizontal_wrapped(|ui| {
                for q in ["Why is my Mac slow?", "What can I safely clean?", "Is my battery healthy?", "Which apps use the most memory?"] {
                    if ui.button(q).clicked() {
                        send = Some(q.to_string());
                    }
                }
            });
        }
        let h = (ui.available_height() - 52.0).max(120.0);
        egui::ScrollArea::vertical().max_height(h).stick_to_bottom(true).show(ui, |ui| {
            for (role, text) in &self.chat.log {
                let user = role == "user";
                egui::Frame::new()
                    .fill(if user { theme::ACCENT.gamma_multiply(0.35) } else { theme::CARD })
                    .corner_radius(12)
                    .inner_margin(10)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(if user { "You" } else { "MacPulse AI" }).small().strong().color(theme::ACCENT_SOFT));
                        ui.label(text);
                    });
            }
            if busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Thinking…");
                });
            }
            if let Some(e) = &self.chat.error {
                ui.label(RichText::new(e).color(theme::BAD));
            }
        });
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.chat.input).hint_text("Ask about your Mac…").desired_width(ui.available_width() - 90.0));
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.add_enabled(!busy, egui::Button::new("Send")).clicked() || enter) && !busy && !self.chat.input.trim().is_empty() {
                send = Some(std::mem::take(&mut self.chat.input));
                r.request_focus();
            }
        });
        if let Some(q) = send {
            self.chat.error = None;
            self.chat.log.push(("user".into(), q));
            let mut msgs: Vec<Msg> = vec![
                ("system".into(), SYSTEM_PROMPT.into()),
                ("user".into(), format!("Current data from the user's Mac (for reference, do not repeat it back):\n{context}")),
                ("assistant".into(), "Understood. What would you like to know?".into()),
            ];
            let n = self.chat.log.len();
            msgs.extend(self.chat.log[n.saturating_sub(8)..].iter().cloned());
            self.chat.rx = Some(spawn_chat(self.settings.clone(), &ctx, msgs));
        }
        open_settings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn cleans_markdown() {
        assert_eq!(clean("## Title\n* **Bold** item"), "Title\n- Bold item");
    }

    #[test]
    fn chat_round_trip_against_mock_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 16384];
            let mut got = 0;
            loop {
                let n = s.read(&mut buf[got..]).unwrap();
                got += n;
                let text = String::from_utf8_lossy(&buf[..got]).to_string();
                if let Some(h) = text.find("\r\n\r\n") {
                    let len = text[..h].lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap())).unwrap_or(0);
                    if got >= h + 4 + len {
                        break;
                    }
                }
            }
            let req = String::from_utf8_lossy(&buf[..got]).to_string();
            let body = r#"{"choices":[{"message":{"role":"assistant","content":"**Looks fine.**"}}]}"#;
            write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            req
        });
        let s = Settings { base_url: format!("http://127.0.0.1:{port}/v1"), model: "m".into(), api_key: "k".into(), ..Default::default() };
        let out = chat(&s, &[("system".into(), "s".into()), ("user".into(), "hello".into())]).unwrap();
        assert_eq!(out, "Looks fine.");
        let req = handle.join().unwrap();
        assert!(req.starts_with("POST /v1/chat/completions"));
        assert!(req.to_lowercase().contains("authorization: bearer k"));
        assert!(req.contains("\"model\":\"m\""));
    }

    #[test]
    fn claude_round_trip_against_mock_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 16384];
            let mut got = 0;
            loop {
                got += s.read(&mut buf[got..]).unwrap();
                let text = String::from_utf8_lossy(&buf[..got]).to_string();
                if let Some(h) = text.find("\r\n\r\n") {
                    let len = text[..h].lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap())).unwrap_or(0);
                    if got >= h + 4 + len {
                        break;
                    }
                }
            }
            let req = String::from_utf8_lossy(&buf[..got]).to_string();
            let body = r#"{"content":[{"type":"text","text":"**All good.**"}]}"#;
            write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            req
        });
        let s = Settings { base_url: format!("http://127.0.0.1:{port}"), model: "claude-x".into(), api_key: "sk-ant-test".into(), kind: "anthropic".into() };
        // Two user turns in a row must be merged, and the system prompt must be top-level.
        let out = chat(&s, &[("system".into(), "be brief".into()), ("user".into(), "a".into()), ("user".into(), "b".into())]).unwrap();
        assert_eq!(out, "All good.");
        let req = handle.join().unwrap();
        assert!(req.starts_with("POST /v1/messages"), "{req}");
        assert!(req.to_lowercase().contains("x-api-key: sk-ant-test"));
        assert!(req.to_lowercase().contains("anthropic-version: 2023-06-01"));
        assert!(req.contains("\"system\":\"be brief\""));
        assert!(req.contains("a\\n\\nb"), "{req}");
    }

    #[test]
    fn claude_without_key_explains() {
        let mut s = Settings::default();
        s.use_claude();
        assert!(chat(&s, &[("user".into(), "x".into())]).unwrap_err().contains("API key"));
        assert!(s.is_claude() && !s.is_local());
    }

    #[test]
    fn unreachable_server_gives_helpful_error() {
        let s = Settings { base_url: "http://127.0.0.1:9/v1".into(), ..Default::default() };
        let e = chat(&s, &[("user".into(), "x".into())]).unwrap_err();
        assert!(e.contains("Can't reach"), "{e}");
    }

    #[test]
    fn locality() {
        let mut s = Settings::default();
        assert!(s.is_local());
        s.base_url = "https://api.example.com/v1".into();
        assert!(!s.is_local());
        assert_eq!(s.host(), "api.example.com");
    }
}
