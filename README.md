<div align="center">

<img src="public/logo.png" alt="Relay Routing logo" width="160" height="160" />

# Relay Routing

**A local-first, multi-provider AI workspace for macOS, Windows, and Linux.**

Chat with local Ollama models or hosted providers, organize work into projects,
attach files, and run durable multi-step research tasks that produce Markdown reports.

[![Platform](https://img.shields.io/badge/platform-macOS%2012%2B%20%7C%20Windows%2010%2B%20%7C%20Linux-black)](https://github.com/akshathsinha08/Relay-Routing)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev/)
[![Rust](https://img.shields.io/badge/Rust-stable-000000?logo=rust)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.9-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)

</div>

---

## Overview

Relay Routing is a desktop app that keeps your conversations, projects, tasks, attachments,
and usage on your machine. Conversations are account-independent: pick a provider and model once,
and eligible accounts are resolved per generation. Everything is stored locally in SQLite,
and API keys live in your operating system's credential store — no telemetry, no cloud sync.

## Features

- **Multiple real providers** — Ollama (local, streaming, model discovery, health checks),
  Anthropic Messages API, OpenRouter (OpenAI-compatible), and Google Gemini (SSE streaming).
- **Account-independent conversations** — choose a provider and model once; eligible accounts
  are resolved per generation. Transient failures retry the same provider without switching
  accounts to evade limits.
- **Interrupted-response continuity** — partial responses are preserved with their original
  generation, attributed segments, and a Continue action that resumes the same response.
- **Persistent workspace** — projects with instructions and shared files, pinned and archived
  conversations, and full-text search over messages.
- **Attachments** — import files into app-managed storage, extract text for prompts, and
  preview images inline.
- **Durable tasks** — a model writes a capped JSON plan, the engine executes steps sequentially
  with checkpoints, supports pause/cancel/resume, and writes a final report.
- **Local by default** — no telemetry, no cloud sync, no cookie or session extraction.
- **Keyboard friendly** — `⌘N`/`Ctrl+N` new conversation, `⌘,`/`Ctrl+,` settings, Enter/⌘Enter (or
  Ctrl+Enter) send modes.

## Providers

| Provider | Transport | Notes |
| --- | --- | --- |
| Ollama | `/api/version`, `/api/tags`, `/api/chat` | Local models; live-verified |
| Anthropic | `/v1/models`, `/v1/messages` | Requires your own API key and billing |
| OpenRouter | `/v1/key`, `/v1/models`, `/v1/chat/completions` | OpenAI-compatible |
| Google Gemini | `/v1beta/models`, `:generateContent` (SSE) | Requires an API key |

All network access happens in the Rust core; the frontend never talks to a provider directly.

## Supported platforms

| Platform | Minimum | Credential store | App data directory |
| --- | --- | --- | --- |
| macOS | 12 (Monterey) | Keychain | `~/Library/Application Support/com.akshathsinha.relayrouting/` |
| Windows | 10 (x64) | Credential Manager | `%APPDATA%\com.akshathsinha.relayrouting\` |
| Linux | glibc-based desktop | Secret Service (e.g. gnome-keyring/KWallet) | `~/.local/share/com.akshathsinha.relayrouting/` |

## Requirements

- macOS 12+, Windows 10+ (x64), or a glibc-based Linux desktop
- Ollama running locally for local models (optional)
- Node 20+ and Rust stable only if building from source
- An API key only if using a hosted provider

Platform-specific notes:

- **Windows** — the NSIS installer downloads and installs the
  [Microsoft Edge WebView2 runtime](https://developer.microsoft.com/microsoft-edge/webview2/)
  automatically if it is not already present (it ships with Windows 11 and current Windows 10
  updates). API keys are stored in Windows Credential Manager.
- **Linux** — a running Secret Service provider (gnome-keyring or KWallet, standard on GNOME
  and KDE desktops) is required to store provider API keys; on minimal window-manager setups
  account secrets cannot be saved.

## Quick start (build from source)

```bash
git clone https://github.com/akshathsinha08/Relay-Routing.git
cd Relay-Routing
npm install
npm run tauri:dev
```

Build prerequisites by platform:

- **macOS** — Xcode Command Line Tools (`xcode-select --install`).
- **Windows** — [Rust with the MSVC toolchain](https://rustup.rs/) plus
  [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/)
  ("Desktop development with C++" workload). The commands above work in PowerShell or cmd.
- **Linux** — Rust plus the WebKitGTK development packages. On Debian/Ubuntu:

  ```bash
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev
  ```

  On Fedora: `sudo dnf install webkit2gtk4.1-devel openssl-devel libxdo-devel
  libappindicator-gtk3-devel`.

## Install (prebuilt)

Prebuilt packages are attached to releases on the GitHub Releases page (currently unsigned;
see [Known limitations](#known-limitations)):

- **Windows** — download `Relay Routing_<version>_x64-setup.exe` (NSIS installer) and run it.
  The installer sets up WebView2 if needed, installs the app into
  `%LOCALAPPDATA%\Programs\Relay Routing\` (per-user, no admin required), and adds a Start-menu
  entry. If you prefer WiX, `Relay Routing_<version>_x64_en-US.msi` is also published.
- **Linux** — install the `.deb` (`sudo apt install ./relay-routing_*.deb`) or run the
  AppImage (`chmod +x RelayRouting_*.AppImage && ./RelayRouting_*.AppImage`). The deb depends
  on `libwebkit2gtk-4.1-0` and `libayatana-appindicator3-1`, which apt resolves automatically.
- **macOS** — open the `.dmg` and drag the app to `/Applications`.

## Build

```bash
npm run tauri:build
```

Installers are written under `src-tauri/target/release/bundle/` for the platform you build on:

- macOS: `dmg/Relay Routing_0.1.0_aarch64.dmg` and `macos/` app bundle
- Windows: `nsis/*.exe` (NSIS setup) and `msi/*.msi` (WiX)
- Linux: `deb/*.deb` and `appimage/*.AppImage`

## Verify

```bash
npm run typecheck      # tsc --noEmit
npm run test           # Vitest unit tests
npm run test:rust      # cargo test
npm run lint:rust      # cargo clippy -D warnings
npm run verify:ollama  # live check against the local Ollama server
```

## Architecture

```
React 19 + TypeScript (Vite)
  stores (zustand) → lib/api.ts (typed invoke wrappers)
        │  Tauri IPC
Rust core (Tauri 2)
  commands → services → providers / database / tasks
        │
SQLite (rusqlite, WAL) + OS credential store + managed files
```

- Chat streaming uses `tauri::ipc::Channel<StreamEvent>`; task progress is polled.
- Secrets live only in the operating system credential store (macOS Keychain, Windows
  Credential Manager, or the Linux Secret Service) under service
  `com.akshathsinha.relayrouting`.
- App data: `relayrouting.sqlite3`, `attachments/`, `artifacts/`, `logs/` under the
  platform directory listed in [Supported platforms](#supported-platforms).

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the data model, routing, and streaming
details.

## Project layout

```
src/                  React frontend (views, features, stores, services)
src-tauri/            Tauri + Rust backend (commands, providers, database, tasks)
  migrations/         SQLite schema migrations
  icons/              App and platform icons
public/               Static assets, including the logo
docs/                 Architecture, development, changelog, project status
scripts/              Utility scripts (icon generation, Ollama verification)
tests/                Frontend test suites
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Development](docs/DEVELOPMENT.md)
- [Project status](docs/PROJECT_STATUS.md)
- [Changelog](docs/CHANGELOG.md)

## Known limitations

- The app is unsigned and not notarized; macOS Gatekeeper warns on other machines, and Windows
  shows a SmartScreen prompt ("More info" → "Run anyway") for the installers.
- Windows and Linux builds are newer than the macOS build and have had less manual testing;
  report issues on the tracker.
- Linux requires a Secret Service provider (gnome-keyring/KWallet) to store API keys.
- No auto-updater or crash reporting.
- Task agent steps use non-streaming completions and no tool use; plans are capped at
  `taskMaxSteps` (default 8).
- Search uses SQL `LIKE`, not FTS; no vector search.
- Single-user local app; no multi-window or multi-user support.
