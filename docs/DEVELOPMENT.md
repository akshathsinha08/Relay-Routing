# Development

## Prerequisites

- Node 20+ (verified with 26.13.0) and npm
- Rust stable with `clippy` (verified with 1.99.1)
- Xcode Command Line Tools
- Ollama (optional, for local models)

## Setup

```bash
npm install
npm run tauri:dev
```

## Commands

| Command | Purpose |
| --- | --- |
| `npm run dev` | Vite dev server only |
| `npm run tauri:dev` | Desktop app with HMR |
| `npm run build` | Typecheck + production frontend bundle |
| `npm run tauri:build` | Desktop app + DMG |
| `npm run typecheck` | `tsc --noEmit` |
| `npm run test` | Vitest unit tests |
| `npm run test:watch` | Vitest watch mode |
| `npm run test:rust` | `cargo test` |
| `npm run lint:rust` | `cargo clippy -- -D warnings` |
| `npm run verify:ollama` | Live health + streaming check against Ollama |

## SDKROOT workaround (this machine)

The installed Command Line Tools default to `MacOSX27.0.sdk`, whose `arm64e.x1-macos`
targets are rejected by the bundled `ld-1267`. Both the repo root and `src-tauri` contain
`.cargo/config.toml` pinning:

```toml
[env]
SDKROOT = { value = "/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk", force = false }
```

Remove this only if your toolchain links against the default SDK.

## Environment

- Ollama base URL defaults to `http://127.0.0.1:11434` (editable in Providers).
- Anthropic key is entered in the UI and stored in the Keychain.
- App data: `~/Library/Application Support/com.akshathsinha.claudemulti/`
  (`claude-multi.sqlite3`, `attachments/`, `artifacts/`, `logs/`).

## Migrations

`src-tauri/migrations/0001_init.sql` is embedded with `include_str!` and applied when
`PRAGMA user_version` is below the target version. Add new files rather than editing v1 once
released, and bump the version check in `database/mod.rs`.

## Testing notes

- Frontend unit tests mock `@/lib/api` and exercise pure helpers plus store behavior.
- Rust tests cover schema seeding, path sanitization, keychain naming, and startup recovery.
- Live provider behavior is verified separately with `verify:ollama` and manual checks in the
  app (Anthropic requires a real key).

## Release

1. Bump versions in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
2. Update `docs/CHANGELOG.md` and `docs/PROJECT_STATUS.md`.
3. Run typecheck, frontend/Rust tests, and clippy.
4. `npm run tauri:build`; artifacts land in `src-tauri/target/release/bundle/`.
5. Signing/notarization is not configured; distribute accordingly.
