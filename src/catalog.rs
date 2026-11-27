//! Smart Care catalog: well-known cache / build-artifact locations under the user's home.
//! Cleaning never deletes the folder itself, only moves its *contents* to the Trash.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Risk {
    /// Regenerated automatically; pre-selected after a scan.
    Safe,
    /// May be expensive to recreate or contain user data; never pre-selected.
    Review,
}

pub struct Target {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub paths: &'static [&'static str],
    pub note: &'static str,
    pub risk: Risk,
}

use Risk::{Review, Safe};

macro_rules! t {
    ($id:expr, $name:expr, $cat:expr, [$($p:expr),+], $note:expr, $risk:expr) => {
        Target { id: $id, name: $name, category: $cat, paths: &[$($p),+], note: $note, risk: $risk }
    };
}

pub static TARGETS: &[Target] = &[
    // Apple developer tools
    t!("xcode-derived", "Xcode DerivedData", "Xcode & Simulators", ["~/Library/Developer/Xcode/DerivedData"], "Build products and indexes; Xcode regenerates them.", Safe),
    t!("xcode-devsupport", "Device Support symbols", "Xcode & Simulators", ["~/Library/Developer/Xcode/iOS DeviceSupport", "~/Library/Developer/Xcode/watchOS DeviceSupport"], "Debug symbols; re-downloaded when a device is connected.", Safe),
    t!("sim-caches", "Simulator caches", "Xcode & Simulators", ["~/Library/Developer/CoreSimulator/Caches"], "Dyld and runtime caches for simulators.", Safe),
    t!("xcode-archives", "Xcode Archives", "Xcode & Simulators", ["~/Library/Developer/Xcode/Archives"], "Archived app builds. You may need them to symbolicate crashes or re-submit.", Review),
    t!("sim-devices", "Simulator devices", "Xcode & Simulators", ["~/Library/Developer/CoreSimulator/Devices"], "Deletes simulator data and installed test apps.", Review),
    // Package managers & build tools
    t!("npm", "npm cache", "Developer Caches", ["~/.npm/_cacache"], "Re-downloaded on next install.", Safe),
    t!("yarn", "Yarn cache", "Developer Caches", ["~/Library/Caches/Yarn"], "Re-downloaded on next install.", Safe),
    t!("pnpm", "pnpm store", "Developer Caches", ["~/Library/pnpm/store"], "Re-downloaded on next install.", Safe),
    t!("pip", "pip cache", "Developer Caches", ["~/Library/Caches/pip"], "Re-downloaded on next install.", Safe),
    t!("homebrew", "Homebrew downloads", "Developer Caches", ["~/Library/Caches/Homebrew"], "Old bottles and downloads.", Safe),
    t!("cocoapods", "CocoaPods cache", "Developer Caches", ["~/Library/Caches/CocoaPods"], "Re-downloaded on next pod install.", Safe),
    t!("swiftpm", "Swift PM cache", "Developer Caches", ["~/Library/Caches/org.swift.swiftpm"], "Re-downloaded on next resolve.", Safe),
    t!("gradle", "Gradle caches", "Developer Caches", ["~/.gradle/caches"], "Re-downloaded on next build; the next build is slower.", Safe),
    t!("cargo", "Cargo registry cache", "Developer Caches", ["~/.cargo/registry/cache"], "Crate archives; re-downloaded when needed.", Safe),
    t!("go-build", "Go build cache", "Developer Caches", ["~/Library/Caches/go-build"], "Rebuilt on next compile.", Safe),
    t!("android", "Android caches", "Developer Caches", ["~/.android/cache"], "Rebuilt on demand.", Safe),
    t!("maven", "Maven repository", "Developer Caches", ["~/.m2/repository"], "Local artifacts; may include locally installed builds that cannot be re-downloaded.", Review),
    // More developer tooling
    t!("xcode-cache", "Xcode app cache", "Xcode & Simulators", ["~/Library/Caches/com.apple.dt.Xcode"], "Rebuilt by Xcode.", Safe),
    t!("android-studio", "Android Studio caches", "Developer Caches", ["~/Library/Caches/Google/AndroidStudio*"], "Indexes rebuilt on next open.", Safe),
    t!("playwright", "Playwright browsers", "Developer Caches", ["~/Library/Caches/ms-playwright"], "Re-downloaded by `npx playwright install`.", Safe),
    t!("cypress", "Cypress binary cache", "Developer Caches", ["~/Library/Caches/Cypress"], "Re-downloaded on next install.", Safe),
    t!("bun", "Bun install cache", "Developer Caches", ["~/.bun/install/cache"], "Re-downloaded on next install.", Safe),
    t!("uv", "uv cache", "Developer Caches", ["~/Library/Caches/uv"], "Re-downloaded on next sync.", Safe),
    t!("composer", "Composer cache", "Developer Caches", ["~/.composer/cache"], "Re-downloaded on next install.", Safe),
    t!("cargo-src", "Cargo extracted sources", "Developer Caches", ["~/.cargo/registry/src", "~/.cargo/git/checkouts"], "Re-extracted on next build.", Safe),
    t!("nuget", "NuGet packages", "Developer Caches", ["~/.nuget/packages"], "Restored on next build, but needs network access.", Review),
    t!("rustup", "Rust toolchains", "Developer Caches", ["~/.rustup/toolchains"], "Removes installed compilers. Only if you want to reinstall.", Review),
    t!("pre-commit", "pre-commit environments", "Developer Caches", ["~/.cache/pre-commit"], "Rebuilt on next hook run.", Safe),
    // Local AI models
    t!("huggingface", "Hugging Face models", "Local AI Models", ["~/.cache/huggingface/hub"], "Downloaded models; re-downloading can be very large.", Review),
    t!("ollama", "Ollama models", "Local AI Models", ["~/.ollama/models"], "Every model you pulled. Re-pulling can take hours.", Review),
    t!("torch", "PyTorch cache", "Local AI Models", ["~/.cache/torch"], "Pretrained weights and hub downloads.", Review),
    t!("lmstudio", "LM Studio models", "Local AI Models", ["~/.lmstudio/models", "~/.cache/lm-studio/models"], "Downloaded GGUF models.", Review),
    // Browsers
    t!("chrome", "Chrome cache", "Browsers", ["~/Library/Caches/Google/Chrome"], "Pages are re-cached as you browse.", Safe),
    t!("safari", "Safari cache", "Browsers", ["~/Library/Caches/com.apple.Safari"], "Pages are re-cached as you browse.", Safe),
    t!("firefox", "Firefox cache", "Browsers", ["~/Library/Caches/Firefox"], "Pages are re-cached as you browse.", Safe),
    t!("edge", "Edge cache", "Browsers", ["~/Library/Caches/Microsoft Edge"], "Pages are re-cached as you browse.", Safe),
    t!("arc", "Arc cache", "Browsers", ["~/Library/Caches/company.thebrowser.Browser"], "Pages are re-cached as you browse.", Safe),
    // Editors
    t!("vscode", "VS Code caches", "Editors & IDEs", ["~/Library/Application Support/Code/CachedData", "~/Library/Application Support/Code/Cache", "~/Library/Application Support/Code/CachedExtensionVSIXs", "~/Library/Caches/com.microsoft.VSCode"], "Rebuilt on next launch.", Safe),
    t!("jetbrains", "JetBrains caches", "Editors & IDEs", ["~/Library/Caches/JetBrains"], "Indexes are rebuilt on next open (slow first launch).", Safe),
    // Apps
    t!("slack", "Slack cache", "App Caches", ["~/Library/Application Support/Slack/Cache", "~/Library/Application Support/Slack/Service Worker/CacheStorage"], "Re-downloaded as you use Slack.", Safe),
    t!("discord", "Discord cache", "App Caches", ["~/Library/Application Support/discord/Cache", "~/Library/Application Support/discord/Code Cache"], "Re-downloaded as you use Discord.", Safe),
    t!("spotify", "Spotify cache", "App Caches", ["~/Library/Caches/com.spotify.client"], "Offline downloads are stored elsewhere.", Safe),
    t!("adobe-media", "Adobe media cache", "App Caches", ["~/Library/Application Support/Adobe/Common/Media Cache Files", "~/Library/Application Support/Adobe/Common/Media Cache"], "Conformed audio/video; Premiere and After Effects rebuild it.", Safe),
    // System
    t!("logs", "App & crash logs", "System", ["~/Library/Logs"], "Diagnostic logs from apps.", Safe),
    t!("mail-downloads", "Mail attachments cache", "System", ["~/Library/Containers/com.apple.mail/Data/Library/Mail Downloads"], "Copies of attachments you opened.", Safe),
    t!("ios-backups", "iPhone/iPad backups", "System", ["~/Library/Application Support/MobileSync/Backup"], "Full device backups. This may be your only copy.", Review),
];

pub struct Command {
    pub title: &'static str,
    pub detail: &'static str,
    pub candidates: &'static [&'static str],
    pub args: &'static [&'static str],
}

/// Things that are not "delete a folder": run the tool's own cleanup so nothing breaks.
pub static COMMANDS: &[Command] = &[
    Command { title: "Docker: prune unused data", detail: "docker system prune -f — removes stopped containers, dangling images and unused networks. Never touches volumes.", candidates: &["/usr/local/bin/docker", "/opt/homebrew/bin/docker", "/Applications/Docker.app/Contents/Resources/bin/docker"], args: &["system", "prune", "-f"] },
    Command { title: "Homebrew: cleanup", detail: "brew cleanup -s — removes old versions and stale downloads.", candidates: &["/opt/homebrew/bin/brew", "/usr/local/bin/brew"], args: &["cleanup", "-s"] },
    Command { title: "Xcode: remove unavailable simulators", detail: "xcrun simctl delete unavailable — deletes simulators whose runtime is gone.", candidates: &["/usr/bin/xcrun"], args: &["simctl", "delete", "unavailable"] },
];
