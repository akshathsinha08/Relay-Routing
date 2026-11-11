<div align="center">

<img src="public/logo.png" alt="Relay Routing logo" width="160" height="160" />

# Relay Routing

**A local-first, multi-provider AI workspace for macOS.**

Chat with local Ollama models or hosted providers, organize work into projects,
attach files, and run durable multi-step research tasks that produce Markdown reports.

[![Platform](https://img.shields.io/badge/platform-macOS%2012%2B-black?logo=apple)](https://www.apple.com/macos/)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev/)
[![Rust](https://img.shields.io/badge/Rust-stable-000000?logo=rust)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.9-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)

</div>

---

## Overview

Relay Routing is a desktop app that keeps your conversations, projects, tasks, attachments,
and usage on your Mac. Conversations are account-independent: pick a provider and model once,
and eligible accounts are resolved per generation. Everything is stored locally in SQLite,
and API keys live in the macOS Keychain — no telemetry, no cloud sync.

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
- **Keyboard friendly** — `⌘N` new conversation, `⌘,` settings, Enter/⌘Enter send modes.

## Providers

| Provider | Transport | Notes |
| --- | --- | --- |
| Ollama | `/api/version`, `/api/tags`, `/api/chat` | Local models; live-verified |
| Anthropic | `/v1/models`, `/v1/messages` | Requires your own API key and billing |
| OpenRouter | `/v1/key`, `/v1/models`, `/v1/chat/completions` | OpenAI-compatible |
| Google Gemini | `/v1beta/models`, `:generateContent` (SSE) | Requires an API key |

All network access happens in the Rust core; the frontend never talks to a provider directly.

## Requirements

- macOS 12+ (built and verified on macOS 26.6 arm64)
- Ollama running locally for local models (optional)
- Node 20+ and Rust stable only if building from source
- An API key only if using a hosted provider

## Quick start

```bash
git clone https://github.com/akshathsinha08/Relay-Routing.git
cd Relay-Routing
npm install
npm run tauri:dev
```

## Build

```bash
npm run tauri:build
```

The unsigned app and DMG are written under `src-tauri/target/release/bundle/`.

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
SQLite (rusqlite, WAL) + macOS Keychain + managed files
```

- Chat streaming uses `tauri::ipc::Channel<StreamEvent>`; task progress is polled.
- Secrets live only in the Keychain under service `com.akshathsinha.relayrouting`.
- App data: `~/Library/Application Support/com.akshathsinha.relayrouting/`
  (`relayrouting.sqlite3`, `attachments/`, `artifacts/`, `logs/`).

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

- The app is unsigned and not notarized; macOS Gatekeeper warns on other machines.
- No auto-updater or crash reporting.
- Task agent steps use non-streaming completions and no tool use; plans are capped at
  `taskMaxSteps` (default 8).
- Search uses SQL `LIKE`, not FTS; no vector search.
- Single-user local app; no multi-window or multi-user support.
