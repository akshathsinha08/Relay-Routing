# Changelog

## Unreleased

### Added

- Account-independent conversations: the saved account is a preference; eligible accounts for
  the fixed provider/model are resolved per generation.
- Additive schema v2 with `generations`, `generation_attempts`, `message_segments`, and
  interrupted-message metadata.
- Coordinated streaming retries for transient transport/5xx/bad-stream failures, capped at two
  total attempts and never used for rate limits or auth failures.
- Interrupted-response Continue: reconstructs context, preserves partial content and segment
  attribution, and resumes the same generation.
- Generation/attempt/segment UI status, account attribution, and startup recovery.
- Mock-provider Rust tests for retry and Continue.

### Changed

- Chat orchestration moved out of `services/chat.rs` into `services/generation.rs`; account
  resolution is shared by chat and tasks.
- Message tokens accumulate across explicit Continue; usage records remain per-run.

## 0.1.0 — 2026-11-09

Initial release.

### Added

- Local-first workspace for macOS built with Tauri 2, React 19, TypeScript, Rust, and SQLite.
- Ollama provider: health checks, model discovery, streaming chat, keep-alive setting.
- Anthropic provider: Messages API streaming, reasoning deltas, usage accounting, key stored
  in the macOS Keychain.
- Persistent conversations with pin, rename, archive, move to project, search, Markdown
  rendering, and export.
- Projects with instructions, shared files, conversation lists, tasks, and artifacts.
- Attachment import with text extraction, image previews, and managed storage.
- Durable task engine: model-generated plans, sequential steps, checkpoints, pause/cancel/
  resume, startup recovery, and Markdown artifacts.
- Usage summary, database backup, data-folder access, theme (system/light/dark), and
  onboarding.
- Automated checks: Vitest unit tests, Rust unit tests, clippy, typecheck, and a live Ollama
  verification script.

### Fixed during development

- Streaming chat buffer races and image-error propagation.
- Path sanitization for Windows-style traversal in attachments.
- SDKROOT linker incompatibility pinned for this toolchain (see DEVELOPMENT.md).
- Dead code and clippy findings removed; test-only helpers gated behind `cfg(test)`.
