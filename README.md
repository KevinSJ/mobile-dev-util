# Mobile Dev Util

A small desktop app (Tauri 2 + Svelte 5) for launching and controlling **Android emulators** and **iOS simulators** without remembering `emulator`, `adb` and `xcrun simctl` incantations.

## Features

| | Android emulator | iOS simulator (macOS) |
| --- | --- | --- |
| List devices with live status | AVDs + running state via `adb` | `simctl list` grouped by runtime |
| Boot / shut down | ✅ (optional cold boot) | ✅ (opens Simulator.app) |
| Wipe data / erase | Wipe data & boot | Erase content & settings |
| Open URL / deep link | `am start -a VIEW` | `simctl openurl` |
| Screenshot | PNG saved to your output folder | PNG saved to your output folder |
| Screen recording | `screenrecord` (max 3 min), pulled to host | `simctl io recordVideo` (H.264) |
| Install app | `.apk` (button or drag & drop) | `.app` bundle (button or drag & drop) |

Other niceties:

- **Finds the tools on its own.** Looks for the Android SDK in `ANDROID_HOME`, `ANDROID_SDK_ROOT` and the standard install locations, and for Xcode in `/usr/bin/xcrun`. That way it works even when launched from Finder or the Dock, where your shell `PATH` isn't available. You can override the SDK path in **Settings**.
- **Activity log** at the bottom shows every command that ran and what it printed.
- Saved screenshots/recordings default to your Desktop; the toast has a **Show in folder** button.
- Shortcuts: `⌘/Ctrl+R` refresh, `⌘/Ctrl+,` settings, `⌘/Ctrl+1` / `2` switch platform, double‑click a device to boot it.

## Requirements

- [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) (Rust toolchain + platform WebView libraries)
- Node.js 20+ and [pnpm](https://pnpm.io/)
- Android: Android SDK with **Platform-Tools** and **Emulator** packages, plus at least one AVD
- iOS: macOS with Xcode installed (`xcode-select --install` / Xcode → Settings → Platforms)

## Development

```bash
pnpm install
pnpm tauri dev        # run the app with hot reload
pnpm check            # svelte-check / TypeScript
cd src-tauri && cargo test   # Rust unit tests (parsers etc.)
```

## Building

```bash
pnpm tauri build
```

The bundle (e.g. `.app` / `.dmg` on macOS) is written to `src-tauri/target/release/bundle/`.

## Project layout

```
src/                     Svelte 5 frontend
  App.svelte             shell: platform switcher, polling, drag & drop
  lib/api.ts             typed wrappers around the Rust commands
  lib/store.svelte.ts    rune-based app state + persisted settings
  lib/components/        DeviceList, DevicePanel, ActivityLog, SettingsDialog, …
src-tauri/src/
  main.rs                Tauri commands
  tools.rs               Android SDK / xcrun discovery
  android.rs             emulator + adb integration
  ios.rs                 xcrun simctl integration
  process.rs             process runner (timeouts, activity events)
```

All process execution happens in Rust. The webview only gets `core` and dialog permissions and cannot run arbitrary shell commands.
