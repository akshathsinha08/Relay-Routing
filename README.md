# Claude Multi

A local-first, multi-provider AI workspace for macOS. Chat with local Ollama models or
Anthropic's API, organize conversations into projects, attach files, and run durable
multi-step research tasks that produce Markdown artifacts.

Everything is stored on your Mac: SQLite for conversations, projects, tasks, attachments,
and usage; the macOS Keychain for API keys.

## Features

- **Two real providers** — Ollama (streaming, model discovery, health checks) and Anthropic
  Messages API (streaming, usage accounting, reasoning deltas).
- **Account-independent conversations** — choose a provider and model once; eligible accounts
  are resolved per generation. Transient failures retry the same provider without switching
  accounts to evade limits.
- **Interrupted-response continuity** — partial responses are preserved with their original
  generation, attributed segments, and a Continue action that resumes the same response.
- **Persistent workspace** — projects with instructions and shared files, pinned and
  archived conversations, full-text search over messages.
- **Attachments** — import files into app-managed storage, extract text for prompts, and
  preview images inline.
- **Durable tasks** — a model writes a capped JSON plan, the engine executes steps
  sequentially with checkpoints, supports pause/cancel/resume, and writes a final report.
- **Local by default** — no telemetry, no cloud sync, no cookie or session extraction.
  Anthropic usage requires your own API key and billing; Claude subscription logins are not
  supported.
- **Keyboard friendly** — `⌘N` new conversation, `⌘,` settings, Enter/⌘Enter send modes.

## Requirements

- macOS 12+ (built and verified on macOS 26.6 arm64)
- Ollama running locally for local models (optional)
- Rust and Node only if building from source
- An Anthropic API key only if using Anthropic

## Run

```bash
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
npm run typecheck
npm run test
npm run test:rust
npm run lint:rust
npm run verify:ollama   # talks to the local Ollama server
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md),
and [docs/PROJECT_STATUS.md](docs/PROJECT_STATUS.md).
