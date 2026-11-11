# Project status

Last updated: 2026-11-11

## State

v0.1.0 plus generation-level account routing and interrupted-response continuity, built and
tested on macOS 26.6 arm64.

- Frontend: React 19 + TypeScript, complete UI (chat, projects, project detail, tasks,
  providers, settings, onboarding).
- Backend: Tauri 2 + Rust + SQLite, complete command surface, streaming, attachments,
  projects, tasks, settings, backups.
- Providers: Ollama (verified live), Anthropic Messages API, OpenRouter (OpenAI-compatible),
  and Google Gemini (implemented; live cloud use needs a real API key).
- Continuity: additive schema v2, generation/attempt/segment records, bounded same-provider
  retry, startup recovery, and explicit Continue.
- Packaging: unsigned `.app` and `.dmg` produced.

## Verification performed

| Check | Result |
| --- | --- |
| `npm run typecheck` | pass |
| `npm run test` | 16/16 pass (format, stores, chat continuity, rendering) |
| `npm run test:rust` | 10/10 pass |
| `npm run lint:rust` | pass (`-D warnings`) |
| `npm run build` | pass |
| `npm run verify:ollama` | pass — 4 models, streaming chat OK |
| `npm run tauri:build` | pass — app 9.51 MiB, DMG 4.43 MiB |
| Launch check | release app launched; live DB migrated from v1 to v2 with integrity OK |
| Mock generation tests | transient 500 retry and interrupted-response Continue pass |
| Frontend continuity tests | Continue stream events, interrupted badge, and segment attribution pass |

Artifacts:

- `src-tauri/target/release/bundle/macos/Claude Multi.app`
- `src-tauri/target/release/bundle/dmg/Claude Multi_0.1.0_aarch64.dmg`

## Known limitations

- The app is unsigned and not notarized; macOS Gatekeeper will warn on other machines.
- No auto-updater or crash reporting.
- Anthropic was not exercised end-to-end because no API key was provided; the request/SSE
  and assistant-prefill continuation paths are implemented and compiled, but live behavior is
  unverified.
- Agent steps use non-streaming `complete` calls and no tool use; plans are capped at
  `taskMaxSteps` (default 8).
- Search uses SQL `LIKE`, not FTS; no vector search.
- The production JS bundle is a single ~660 kB chunk (Vite warns); acceptable for a desktop
  app but could be split later.
- Ollama chat verification used `gemma4:12b` and returned an empty short reply with
  `num_predict: 16`; streaming and event parsing were confirmed, output quality was not.
- Tokens/costs are tracked as counts only; no pricing table.
- Single-user local app; no multi-window or multi-user support.

## Next steps (optional)

1. Split the bundle and lazy-load heavy views.
2. Add an Anthropic live integration test gated by an env key.
3. Add FTS5 index and message pagination for very long conversations.
4. Configure Developer ID signing and notarization for distribution.
