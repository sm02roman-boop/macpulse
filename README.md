# MacPulse

A native-feeling Mac system monitor and storage cleaner written in Rust (egui). One binary, no
daemons, nothing runs unless the window is open.

## Build and run (on your Mac)

```bash
# one-time: install Rust from https://rustup.rs, then:
cargo run --release                 # run directly
./scripts/build_app.sh              # universal MacPulse.app in dist/
./scripts/make_dmg.sh               # same, packaged as dist/MacPulse.dmg
cp -R dist/MacPulse.app /Applications/
```

Requires Rust 1.85+ and Xcode Command Line Tools (`xcode-select --install`).

## What's inside

| Page | What it does |
|---|---|
| Overview | Live CPU, memory, network, disk and battery with 90 s rolling charts |
| Memory | Squarified treemap of resident memory, helper processes grouped under their app, one-click Quit |
| CPU | Per-core load (E/P cores on Apple Silicon), top apps |
| Storage | Disk analyzer: proportional treemap, drill down, Reveal in Finder, Move to Trash |
| Smart Care | 46 cache/build locations (Xcode, npm, Gradle, browsers, AI models...). Contents go to the Trash. Per-item Ignore list. Docker / Homebrew / simulator cleanups. Empty Trash (permanent, confirmed) |
| App Leftovers | Orphaned `~/Library` data for apps that are no longer installed (matched by bundle ID) |
| Uninstaller | Trash an app plus every matching `~/Library` file for its bundle ID |
| Startup Items | LaunchAgents/Daemons audit; flags items whose program no longer exists |
| Network | Per-app throughput via `nettop` |
| Battery | Charge, health, cycles, temperature, wattage (`ioreg`) and per-process energy impact (`top`) |
| Health | Rule-based audit: memory pressure, swap, low disk, thermal throttling, sustained CPU, memory-growth trend, one-click Quit |

## AI explanations (free, on your Mac)

Every page has an **Explain with AI** card, and there is an **Assistant** page for free-form questions.
It uses a local model through [Ollama](https://ollama.com), so it costs nothing and your data stays on the Mac.

```bash
# after installing Ollama from ollama.com/download and opening it once:
ollama pull llama3.2:3b
```

Then open **AI Settings** in the app and click *Save & test*. Any OpenAI-compatible server works
(LM Studio, or a hosted API with a key). If the server is not on your Mac, the app shows an orange
"sends data" badge, because the page's app names and file names are sent to it when you click Explain.
Settings live in `~/.macpulse/settings.json`.

**Other free options (no credit card):** AI Settings has one-click presets for Groq, Google Gemini and
OpenRouter free models. Create a free key on the provider's site, paste it, press *Save & test*.

**Using Claude instead:** in AI Settings click *Claude (Anthropic API)*, paste an API key from
console.anthropic.com and press *Save & test*. A claude.ai subscription does not include API access;
the API is billed separately by usage. The key is stored in `~/.macpulse/settings.json` (mode 600).

Not Apple's on-device model: that framework is Swift-only and needs macOS 26 on Apple Silicon.

## Appearance

On macOS the window uses a glass look: the system blur behind the window, frosted cards and a title bar
merged into the content. If it looks wrong on your Mac, open **AI Settings > Appearance**, untick
*Glass window* and restart for a solid dark window.

## Permissions

For Storage and Leftovers to see everything in `~/Library`, grant **Full Disk Access**:
System Settings > Privacy & Security > Full Disk Access > add MacPulse.app.

## Safety model

- Nothing is permanently deleted. Everything is moved to the Trash via `NSFileManager`.
- Smart Care only touches the *contents* of known folders, and refuses any path that is not at
  least two levels inside your home folder.
- "Review" items (Ollama models, Xcode Archives, iPhone backups...) are never pre-selected.
- Leftovers are never pre-selected.

## Known limits

- The menu bar item (live CPU %, memory, disk, network, battery) exists only while the window is open or minimized. Closing the window quits the app.
- Not implemented: IOKit PMU power telemetry, microsecond Mach CPU sampling, notarization, auto-update.
- Memory "used" comes from the `sysinfo` crate; it can differ by a few percent from Activity Monitor.
- Per-core/P-E labelling assumes Apple Silicon enumerates efficiency cores first.
- Processes owned by root are not visible to a normal user app, so their memory is not counted
  in the treemap.
- Smart Care's catalog only covers the locations listed in `src/catalog.rs`; add your own there.

## Development

```bash
cargo test    # unit tests + a headless render of every page
```
