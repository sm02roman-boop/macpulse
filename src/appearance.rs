use crate::util;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    /// Translucent window with macOS blur behind it. Applied at startup.
    pub glass: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self { glass: true }
    }
}

impl Appearance {
    fn path() -> std::path::PathBuf {
        util::home().join(".macpulse").join("appearance.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, serde_json::to_string_pretty(self).unwrap_or_default());
    }
}
