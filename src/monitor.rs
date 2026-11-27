use crate::mac::{self, Battery};
use eframe::egui;
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, ProcessesToUpdate, System};

pub const HISTORY: usize = 90;

#[derive(Clone, Debug)]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
    pub group: String,
    pub rss: u64,
    pub cpu: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub cores: Vec<f32>,
    pub cpu_total: f32,
    pub mem_total: u64,
    pub mem_used: u64,
    pub swap_used: u64,
    pub disk_total: u64,
    pub disk_free: u64,
    pub rx_rate: f64,
    pub tx_rate: f64,
    pub pressure: u8,
    pub speed_limit: Option<u8>,
    pub battery: Option<Battery>,
    pub procs: Vec<ProcInfo>,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub name: String,
    pub rss: u64,
    pub cpu: f32,
    pub pids: Vec<u32>,
}

/// "/Applications/Google Chrome.app/Contents/..." -> "Google Chrome"
pub fn app_group(exe: Option<&std::path::Path>, name: &str) -> String {
    if let Some(p) = exe {
        for c in p.components() {
            let s = c.as_os_str().to_string_lossy();
            if let Some(app) = s.strip_suffix(".app") {
                return app.to_string();
            }
        }
    }
    name.to_string()
}

/// Background sampler: one thread, one sample per second, zero work while the app is closed.
pub fn spawn(ctx: egui::Context) -> Receiver<Snapshot> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let mut sys = System::new();
        let mut nets = Networks::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        sys.refresh_cpu_usage();
        let mut tick = 0u64;
        let mut battery = None;
        let mut pressure = 1u8;
        let mut speed_limit = None;
        let mut last = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let dt = last.elapsed().as_secs_f64().max(0.001);
            last = Instant::now();

            sys.refresh_cpu_usage();
            sys.refresh_memory();
            sys.refresh_processes(ProcessesToUpdate::All, true);
            nets.refresh(true);
            if tick % 5 == 0 {
                disks.refresh(true);
                battery = mac::battery();
                pressure = mac::memory_pressure();
                speed_limit = mac::cpu_speed_limit();
            }
            tick += 1;

            let (mut rx_b, mut tx_b) = (0u64, 0u64);
            for (name, n) in nets.iter() {
                if name.starts_with("lo") {
                    continue;
                }
                rx_b += n.received();
                tx_b += n.transmitted();
            }

            let disk = disks
                .iter()
                .find(|d| d.mount_point() == std::path::Path::new("/"))
                .or_else(|| disks.iter().max_by_key(|d| d.total_space()));

            let procs = sys
                .processes()
                .iter()
                .map(|(pid, p)| {
                    let name = p.name().to_string_lossy().into_owned();
                    ProcInfo {
                        pid: pid.as_u32(),
                        group: app_group(p.exe(), &name),
                        name,
                        rss: p.memory(),
                        cpu: p.cpu_usage(),
                    }
                })
                .collect();

            let snap = Snapshot {
                cores: sys.cpus().iter().map(|c| c.cpu_usage()).collect(),
                cpu_total: sys.global_cpu_usage(),
                mem_total: sys.total_memory(),
                mem_used: sys.used_memory(),
                swap_used: sys.used_swap(),
                disk_total: disk.map(|d| d.total_space()).unwrap_or(0),
                disk_free: disk.map(|d| d.available_space()).unwrap_or(0),
                rx_rate: rx_b as f64 / dt,
                tx_rate: tx_b as f64 / dt,
                pressure,
                speed_limit,
                battery: battery.clone(),
                procs,
            };
            if tx.send(snap).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    });
    rx
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Ok,
    Warn,
    Critical,
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    /// App group the user can quit as a one-click remediation.
    pub group: Option<String>,
}

/// UI-side state: latest snapshot, rolling 90 s history, grouped processes and health audit.
pub struct MonState {
    pub snap: Snapshot,
    pub cpu: VecDeque<f32>,
    pub mem: VecDeque<f32>,
    pub rx: VecDeque<f32>,
    pub tx: VecDeque<f32>,
    pub groups: Vec<Group>,
    pub findings: Vec<Finding>,
    pub perf_cores: usize,
    pub eff_cores: usize,
    ticks: u64,
    hot_streak: HashMap<String, u32>,
    rss_hist: HashMap<String, VecDeque<u64>>,
}

fn push_hist(q: &mut VecDeque<f32>, v: f32) {
    if q.len() >= HISTORY {
        q.pop_front();
    }
    q.push_back(v);
}

impl MonState {
    pub fn new() -> Self {
        let (p, e) = mac::core_layout();
        Self {
            snap: Snapshot::default(),
            cpu: VecDeque::new(),
            mem: VecDeque::new(),
            rx: VecDeque::new(),
            tx: VecDeque::new(),
            groups: vec![],
            findings: vec![],
            perf_cores: p,
            eff_cores: e,
            ticks: 0,
            hot_streak: HashMap::new(),
            rss_hist: HashMap::new(),
        }
    }

    pub fn push(&mut self, s: Snapshot) {
        push_hist(&mut self.cpu, s.cpu_total);
        let mem_pct = if s.mem_total > 0 {
            s.mem_used as f32 / s.mem_total as f32 * 100.0
        } else {
            0.0
        };
        push_hist(&mut self.mem, mem_pct);
        push_hist(&mut self.rx, s.rx_rate as f32);
        push_hist(&mut self.tx, s.tx_rate as f32);

        let mut map: HashMap<&str, Group> = HashMap::new();
        for p in &s.procs {
            let g = map.entry(p.group.as_str()).or_insert_with(|| Group {
                name: p.group.clone(),
                rss: 0,
                cpu: 0.0,
                pids: vec![],
            });
            g.rss += p.rss;
            g.cpu += p.cpu;
            g.pids.push(p.pid);
        }
        let mut groups: Vec<Group> = map.into_values().collect();
        groups.sort_by(|a, b| b.rss.cmp(&a.rss));
        self.groups = groups;
        self.snap = s;
        self.ticks += 1;
        self.analyze();
    }

    /// Deterministic rule-based audit (no AI involved).
    fn analyze(&mut self) {
        let s = &self.snap;
        let mut out: Vec<Finding> = vec![];
        let f = |sev, t: &str, d: String, g: Option<String>| Finding {
            severity: sev,
            title: t.into(),
            detail: d,
            group: g,
        };

        match s.pressure {
            4.. => out.push(f(Severity::Critical, "Memory pressure is critical",
                "macOS is compressing and swapping aggressively. Quit the largest apps below.".into(), None)),
            2..=3 => out.push(f(Severity::Warn, "Memory pressure is elevated",
                "The system is starting to compress memory. Close apps you are not using.".into(), None)),
            _ => {}
        }
        if let Some(lim) = s.speed_limit {
            if lim < 100 {
                out.push(f(Severity::Warn, "CPU is being thermally throttled",
                    format!("macOS has limited the CPU to {lim}% of full speed to cool down. Check for a runaway process and good airflow."), None));
            }
        }
        if s.swap_used > 4 * 1024 * 1024 * 1024 {
            out.push(f(Severity::Warn, "Heavy swap usage",
                format!("{} of swap in use. Expect stutter when switching apps.", crate::util::bytes(s.swap_used)), None));
        }
        if s.disk_total > 0 {
            let frac = s.disk_free as f64 / s.disk_total as f64;
            if frac < 0.05 {
                out.push(f(Severity::Critical, "Startup disk almost full",
                    format!("Only {} free. macOS needs headroom for swap and updates. Run Smart Care.", crate::util::size(s.disk_free)), None));
            } else if frac < 0.10 {
                out.push(f(Severity::Warn, "Startup disk is getting full",
                    format!("{} free ({:.0}%). Run Smart Care to reclaim space.", crate::util::size(s.disk_free), frac * 100.0), None));
            }
        }
        if let Some(b) = &s.battery {
            if let Some(h) = b.health_percent {
                if h < 80.0 {
                    out.push(f(Severity::Warn, "Battery capacity has degraded",
                        format!("Maximum capacity is about {h:.0}% of design capacity."), None));
                }
            }
        }

        // Sustained CPU hogs (>80% of one core for 30 s).
        let mut hot: HashMap<String, u32> = HashMap::new();
        for g in &self.groups {
            if g.cpu > 80.0 {
                let n = self.hot_streak.get(&g.name).copied().unwrap_or(0) + 1;
                hot.insert(g.name.clone(), n);
                if n >= 30 {
                    out.push(f(Severity::Warn, &format!("{} is using sustained CPU", g.name),
                        format!("{:.0}% CPU for {n} seconds.", g.cpu), Some(g.name.clone())));
                }
            }
        }
        self.hot_streak = hot;

        // Memory growth trend: sample every 30 s, keep 15 min.
        if self.ticks % 30 == 0 {
            for g in self.groups.iter().take(40) {
                let h = self.rss_hist.entry(g.name.clone()).or_default();
                if h.len() >= 30 {
                    h.pop_front();
                }
                h.push_back(g.rss);
            }
            let live: std::collections::HashSet<&str> =
                self.groups.iter().map(|g| g.name.as_str()).collect();
            self.rss_hist.retain(|k, _| live.contains(k.as_str()));
        }
        for (name, h) in &self.rss_hist {
            if h.len() < 10 {
                continue;
            }
            let (first, last) = (h[0], *h.back().unwrap());
            let rising = h.iter().zip(h.iter().skip(1)).filter(|(a, b)| b >= a).count();
            if last > first + 400 * 1024 * 1024 && rising as f32 / (h.len() - 1) as f32 >= 0.7 {
                out.push(f(Severity::Warn, &format!("{name} may be leaking memory"),
                    format!("Grew from {} to {} over {} minutes with a steady upward trend.",
                        crate::util::bytes(first), crate::util::bytes(last), h.len() / 2), Some(name.clone())));
            }
        }

        if out.is_empty() {
            out.push(f(Severity::Ok, "No issues detected",
                "Memory pressure, swap, disk space and CPU all look healthy.".into(), None));
        }
        out.sort_by_key(|x| std::cmp::Reverse(x.severity as u8));
        self.findings = out;
    }
}
