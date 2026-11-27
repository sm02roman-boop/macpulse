use std::path::PathBuf;

/// Binary units (matches how RAM is usually discussed).
pub fn bytes(b: u64) -> String {
    scale(b, 1024.0, &["B", "KB", "MB", "GB", "TB"])
}

/// Decimal units (matches Finder / disk sizes).
pub fn size(b: u64) -> String {
    scale(b, 1000.0, &["B", "KB", "MB", "GB", "TB"])
}

pub fn rate(bps: f64) -> String {
    format!("{}/s", size(bps.max(0.0) as u64))
}

fn scale(b: u64, base: f64, units: &[&str]) -> String {
    let mut v = b as f64;
    let mut i = 0;
    while v >= base && i < units.len() - 1 {
        v /= base;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else {
        format!("{v:.1} {}", units[i])
    }
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

pub fn expand(p: &str) -> PathBuf {
    match p.strip_prefix("~/") {
        Some(rest) => home().join(rest),
        None => PathBuf::from(p),
    }
}

/// Deterministic hue from a string, so each app keeps its colour between frames/launches.
pub fn hue(s: &str) -> f32 {
    let mut h: u32 = 5381;
    for b in s.bytes() {
        h = h.wrapping_mul(33) ^ b as u32;
    }
    (h % 360) as f32 / 360.0
}

/// Move one path to the Trash (reversible). On macOS use NSFileManager, which avoids
/// the Finder Automation prompt and the trash sound.
pub fn move_to_trash(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        let mut ctx = trash::TrashContext::default();
        ctx.set_delete_method(DeleteMethod::NsFileManager);
        ctx.delete(path).map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        trash::delete(path).map_err(|e| e.to_string())
    }
}

/// Safety rail: only ever clean paths that are strictly inside the user's home,
/// at least two levels deep (never `~` or `~/Library` themselves).
pub fn is_safe_clean_path(p: &std::path::Path) -> bool {
    let home = home();
    if home.as_os_str().is_empty() || p.components().any(|c| c.as_os_str() == "..") {
        return false;
    }
    match p.strip_prefix(&home) {
        Ok(rel) => rel.components().count() >= 2,
        Err(_) => false,
    }
}

/// Expand `~` and a single `*` wildcard in the last path component
/// (e.g. `~/Library/Caches/Google/AndroidStudio*`). Returns only paths that exist.
pub fn expand_glob(p: &str) -> Vec<PathBuf> {
    let full = expand(p);
    let Some(last) = full.file_name().map(|n| n.to_string_lossy().into_owned()) else { return vec![] };
    let Some((pre, suf)) = last.split_once('*') else {
        return if full.exists() { vec![full] } else { vec![] };
    };
    let Some(parent) = full.parent() else { return vec![] };
    let Ok(rd) = std::fs::read_dir(parent) else { return vec![] };
    rd.flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.len() >= pre.len() + suf.len() && n.starts_with(pre) && n.ends_with(suf)
        })
        .map(|e| e.path())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches_prefix() {
        let d = std::env::temp_dir().join(format!("macpulse-glob-{}", std::process::id()));
        std::fs::create_dir_all(d.join("AndroidStudio2024.1")).unwrap();
        std::fs::create_dir_all(d.join("AndroidStudio2025.2")).unwrap();
        std::fs::create_dir_all(d.join("Other")).unwrap();
        let pat = format!("{}/AndroidStudio*", d.display());
        assert_eq!(expand_glob(&pat).len(), 2);
        std::fs::remove_dir_all(&d).ok();
    }
}
