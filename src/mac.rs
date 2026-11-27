//! macOS-specific data sources. Everything shells out to stock system tools
//! (`ioreg`, `sysctl`, `nettop`) so the rest of the app stays portable; on other
//! platforms these simply return `None`/empty and the UI explains why.

use std::collections::HashMap;
use std::process::{Command, Stdio};

fn run(prog: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(prog)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn sysctl_i64(name: &str) -> Option<i64> {
    run("sysctl", &["-n", name])?.trim().parse().ok()
}

/// (performance cores, efficiency cores). (0,0) on Intel / non-mac.
pub fn core_layout() -> (usize, usize) {
    (
        sysctl_i64("hw.perflevel0.logicalcpu").unwrap_or(0) as usize,
        sysctl_i64("hw.perflevel1.logicalcpu").unwrap_or(0) as usize,
    )
}

/// 1 = normal, 2 = warning, 4 = critical (kern.memorystatus_vm_pressure_level).
pub fn memory_pressure() -> u8 {
    sysctl_i64("kern.memorystatus_vm_pressure_level").unwrap_or(1) as u8
}

#[derive(Clone, Debug, Default)]
pub struct Battery {
    pub percent: f32,
    pub charging: bool,
    pub plugged_in: bool,
    pub cycles: Option<u32>,
    pub health_percent: Option<f32>,
    pub temperature_c: Option<f32>,
    pub voltage_v: Option<f32>,
    pub watts: Option<f32>, // + charging, - discharging
    pub minutes_remaining: Option<u32>,
}

pub fn battery() -> Option<Battery> {
    let text = run("ioreg", &["-rn", "AppleSmartBattery", "-w0"])?;
    parse_battery(&text)
}

pub fn parse_battery(text: &str) -> Option<Battery> {
    let mut kv: HashMap<&str, &str> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix('"') {
            if let Some((k, v)) = rest.split_once("\" = ") {
                kv.entry(k).or_insert(v.trim());
            }
        }
    }
    if kv.is_empty() {
        return None;
    }
    let num = |k: &str| kv.get(k).and_then(|v| v.parse::<u64>().ok());
    let yes = |k: &str| kv.get(k).map(|v| *v == "Yes").unwrap_or(false);

    let cur = num("CurrentCapacity")? as f32;
    let max = num("MaxCapacity").unwrap_or(100).max(1) as f32;
    let raw_max = num("AppleRawMaxCapacity").or(num("MaxCapacity"));
    let design = num("DesignCapacity");
    let health = match (raw_max, design) {
        (Some(r), Some(d)) if d > 0 && r > 100 => Some(r as f32 / d as f32 * 100.0),
        _ => None,
    };
    let voltage = num("Voltage").map(|v| v as f32 / 1000.0);
    // Amperage is a signed 64-bit value that ioreg prints as unsigned.
    let amps = num("Amperage").map(|a| (a as i64) as f32 / 1000.0);
    let watts = match (voltage, amps) {
        (Some(v), Some(a)) => Some(v * a),
        _ => None,
    };
    let remaining = num("TimeRemaining").filter(|m| *m < 6000).map(|m| m as u32);

    Some(Battery {
        percent: cur / max * 100.0,
        charging: yes("IsCharging"),
        plugged_in: yes("ExternalConnected"),
        cycles: num("CycleCount").map(|c| c as u32),
        health_percent: health,
        temperature_c: num("Temperature").map(|t| t as f32 / 100.0),
        voltage_v: voltage,
        watts,
        minutes_remaining: remaining,
    })
}

/// Per-process network throughput over ~1s: (process, bytes_in/s, bytes_out/s).
pub fn nettop() -> Vec<(String, f64, f64)> {
    let Some(text) = run(
        "nettop",
        &["-P", "-d", "-x", "-L", "2", "-s", "1", "-J", "bytes_in,bytes_out"],
    ) else {
        return vec![];
    };
    parse_nettop(&text)
}

pub fn parse_nettop(text: &str) -> Vec<(String, f64, f64)> {
    let mut samples: Vec<Vec<(String, f64, f64)>> = vec![];
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        if line.starts_with(',') || line.contains("bytes_in") {
            samples.push(vec![]);
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 {
            continue;
        }
        let (Ok(i), Ok(o)) = (parts[1].parse::<f64>(), parts[2].parse::<f64>()) else {
            continue;
        };
        // "Safari.1234" -> "Safari"
        let name = match parts[0].rsplit_once('.') {
            Some((n, pid)) if pid.chars().all(|c| c.is_ascii_digit()) => n,
            _ => parts[0],
        };
        if let Some(s) = samples.last_mut() {
            s.push((name.to_string(), i, o));
        }
    }
    let mut last = samples.pop().unwrap_or_default();
    last.retain(|(_, i, o)| i + o > 0.0);
    last.sort_by(|a, b| (b.1 + b.2).total_cmp(&(a.1 + a.2)));
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_parse() {
        let t = r#"
    "CurrentCapacity" = 80
    "MaxCapacity" = 100
    "AppleRawMaxCapacity" = 5000
    "DesignCapacity" = 6000
    "CycleCount" = 212
    "IsCharging" = No
    "ExternalConnected" = Yes
    "Temperature" = 3050
    "Voltage" = 12000
    "Amperage" = 18446744073709550616
    "TimeRemaining" = 65535
"#;
        let b = parse_battery(t).unwrap();
        assert_eq!(b.percent.round(), 80.0);
        assert_eq!(b.health_percent.map(|h| h.round()), Some(83.0));
        assert!(b.watts.unwrap() < 0.0); // negative amperage = discharging
        assert_eq!(b.minutes_remaining, None);
        assert_eq!(b.cycles, Some(212));
    }

    #[test]
    fn nettop_parse() {
        let t = ",bytes_in,bytes_out,\nSafari.123,1,2,\n,bytes_in,bytes_out,\nSafari.123,1000,200,\nidle.5,0,0,\n";
        let r = parse_nettop(t);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, "Safari");
        assert_eq!(r[0].1, 1000.0);
    }
}

/// Politely ask processes to quit (SIGTERM). Only works for processes you own.
pub fn terminate(pids: &[u32]) {
    if pids.is_empty() {
        return;
    }
    let _ = Command::new("kill")
        .arg("-TERM")
        .args(pids.iter().map(|p| p.to_string()))
        .stderr(Stdio::null())
        .status();
}

/// Per-app "energy impact" from `top` (the same figure Activity Monitor's Energy tab shows).
pub fn top_power() -> Vec<(String, f32)> {
    let Some(text) = run("top", &["-l", "2", "-n", "15", "-o", "power", "-stats", "pid,command,power", "-s", "1"]) else {
        return vec![];
    };
    parse_top_power(&text)
}

pub fn parse_top_power(text: &str) -> Vec<(String, f32)> {
    // Two samples are printed; only the last one is a real measurement.
    let Some(start) = text.rfind("PID") else { return vec![] };
    let mut out = vec![];
    for line in text[start..].lines().skip(1) {
        let toks: Vec<&str> = line.split_whitespace().collect();
        if toks.len() < 3 || toks[0].parse::<u32>().is_err() {
            continue;
        }
        let Ok(power) = toks[toks.len() - 1].parse::<f32>() else { continue };
        let name = toks[1..toks.len() - 1].join(" ");
        if power > 0.0 {
            out.push((name, power));
        }
    }
    out.sort_by(|a, b| b.1.total_cmp(&a.1));
    out
}

/// Thermal throttling: CPU speed limit in percent (100 = not throttled).
pub fn cpu_speed_limit() -> Option<u8> {
    let text = run("pmset", &["-g", "therm"])?;
    text.lines()
        .find(|l| l.contains("CPU_Speed_Limit"))
        .and_then(|l| l.split('=').nth(1))
        .and_then(|v| v.trim().parse().ok())
}

#[cfg(test)]
mod more_tests {
    use super::*;

    #[test]
    fn top_power_parse() {
        let t = "PID    COMMAND          POWER\n1      old              99.0\n\nPID    COMMAND          POWER\n123    Google Chrome H  12.5\n45     WindowServer     3.1\n9      idle             0.0\n";
        let r = parse_top_power(t);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, "Google Chrome H");
        assert!((r[0].1 - 12.5).abs() < 0.01);
    }
}
