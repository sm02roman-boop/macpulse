use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Allocated size on disk (what Finder calls "size on disk"), not following symlinks.
pub fn size_of(path: &Path, cancel: Option<&AtomicBool>) -> u64 {
    let Ok(md) = std::fs::symlink_metadata(path) else { return 0 };
    if md.file_type().is_symlink() {
        return 0;
    }
    if !md.is_dir() {
        return alloc(&md);
    }
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if cancel.map(|c| c.load(Ordering::Relaxed)).unwrap_or(false) {
            return total;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let Ok(md) = e.metadata() else { continue };
            let ft = md.file_type();
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                stack.push(e.path());
            } else {
                total += alloc(&md);
            }
        }
    }
    total
}

#[cfg(unix)]
fn alloc(md: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    md.blocks() * 512
}
#[cfg(not(unix))]
fn alloc(md: &std::fs::Metadata) -> u64 {
    md.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_nested_files() {
        let d = std::env::temp_dir().join(format!("macpulse-test-{}", std::process::id()));
        std::fs::create_dir_all(d.join("a/b")).unwrap();
        std::fs::write(d.join("a/b/f.bin"), vec![1u8; 100_000]).unwrap();
        std::fs::write(d.join("g.bin"), vec![1u8; 50_000]).unwrap();
        let s = size_of(&d, None);
        assert!(s >= 150_000 && s < 400_000, "{s}");
        std::fs::remove_dir_all(&d).ok();
    }
}
