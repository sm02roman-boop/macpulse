//! Plain-text summaries of what each page shows, used as the AI's source of truth.

use crate::monitor::{MonState, Severity};
use crate::util;
use std::fmt::Write;

pub fn system(m: &MonState, top: usize) -> String {
    let s = &m.snap;
    let mut o = String::new();
    let mem_pct = if s.mem_total > 0 { s.mem_used as f64 / s.mem_total as f64 * 100.0 } else { 0.0 };
    let _ = writeln!(o, "CPU: {:.0}% overall, {} cores{}", s.cpu_total, s.cores.len(),
        if m.perf_cores > 0 { format!(" ({} performance + {} efficiency)", m.perf_cores, m.eff_cores) } else { String::new() });
    let _ = writeln!(o, "Memory: {} of {} used ({:.0}%), swap used {}, pressure: {}", util::bytes(s.mem_used), util::bytes(s.mem_total), mem_pct,
        util::bytes(s.swap_used), match s.pressure { 4.. => "critical", 2..=3 => "elevated", _ => "normal" });
    let _ = writeln!(o, "Startup disk: {} free of {}", util::size(s.disk_free), util::size(s.disk_total));
    let _ = writeln!(o, "Network: download {}, upload {}", util::rate(s.rx_rate), util::rate(s.tx_rate));
    if let Some(l) = s.speed_limit {
        let _ = writeln!(o, "Thermal: CPU speed limit {l}%");
    }
    if let Some(b) = &s.battery {
        let _ = writeln!(o, "Battery: {:.0}%{}{}{}", b.percent, if b.charging { ", charging" } else if b.plugged_in { ", plugged in" } else { ", on battery" },
            b.health_percent.map(|h| format!(", capacity {h:.0}% of design")).unwrap_or_default(),
            b.cycles.map(|c| format!(", {c} cycles")).unwrap_or_default());
    }
    let _ = writeln!(o, "Top apps by memory:");
    for g in m.groups.iter().take(top) {
        let _ = writeln!(o, "- {}: {} ({} processes, {:.0}% CPU)", g.name, util::bytes(g.rss), g.pids.len(), g.cpu);
    }
    let mut by_cpu: Vec<_> = m.groups.iter().collect();
    by_cpu.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
    let _ = writeln!(o, "Top apps by CPU:");
    for g in by_cpu.iter().take(top.min(6)) {
        let _ = writeln!(o, "- {}: {:.0}% CPU", g.name, g.cpu);
    }
    let _ = writeln!(o, "Automatic findings:");
    for f in &m.findings {
        let sev = match f.severity { Severity::Critical => "critical", Severity::Warn => "warning", Severity::Ok => "ok" };
        let _ = writeln!(o, "- [{sev}] {}: {}", f.title, f.detail);
    }
    o
}
