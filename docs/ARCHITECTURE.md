# Architecture

## Layers

```
React 19 + TypeScript (Vite)
  stores (zustand) → lib/api.ts (typed invoke wrappers)
        │  Tauri IPC
Rust core (Tauri 2)
  commands → services → providers / database / tasks
        │
SQLite (rusqlite, WAL) + macOS Keychain + managed files
```

- The frontend never talks to a provider directly. All network access happens in Rust.
- Commands are sync unless they do network I/O or streaming; those are async.
- Chat streaming uses `tauri::ipc::Channel<StreamEvent>`; the frontend creates a channel,
  calls `send_message` or `continue_generation`, and receives `started`, `attemptStarted`,
  `segmentStarted`, `delta`, `reasoning`, `retryScheduled`, `usage`, `interrupted`, and
  `done` events.
- Task progress is polled (`get_task_detail`, ~1.2 s) rather than streamed.

## Data model (schema v2)

`providers`, `accounts`, `projects`, `project_attachments`, `conversations`, `messages`,
`generations`, `generation_attempts`, `message_segments`, `attachments`, `tasks`,
`task_steps`, `checkpoints`, `task_pauses`, `artifacts`, `usage`, `settings`.

Notable constraints:

- `messages.status` ∈ `complete | streaming | error | cancelled`.
- `generations.status` ∈ `pending | streaming | completed | interrupted | failed | cancelled`;
  `message_segments.status` ∈ `streaming | complete | interrupted | failed`.
- Interrupted assistant messages keep partial content plus `interrupted` and
  `interruption_reason`; Continue adds a new attempt to the same generation.
- Foreign keys are enforced; deleting a project/conversation cascades to owned rows.
- Settings are a key/value table with a server-side allowlist and range checks.
- Secrets live only in the Keychain, under service `com.akshathsinha.relayrouting` and
  username `provider-account:<account_id>`.

## Providers

`ProviderKind::Ollama`, `ProviderKind::Anthropic`, `ProviderKind::OpenRouter`, and
`ProviderKind::Gemini` implement a common trait:

- `health` — reachability, version, latency
- `list_models` — normalized `ModelInfo` (capabilities, size, family)
- `stream` — emits `StreamEvent` into a channel
- `complete` — non-streaming completion used by the task engine

Ollama uses `/api/version`, `/api/tags`, `/api/chat`. Anthropic uses `/v1/models` and
`/v1/messages`. OpenRouter uses the OpenAI-compatible `/v1/key`, `/v1/models`, and
`/v1/chat/completions`. Gemini uses `/v1beta/models` and
`/v1beta/models/{model}:generateContent` (SSE via `alt=sse`). Cloud providers use SSE
parsing. Errors carry stable codes (`provider_unreachable`, `invalid_api_key`,
`rate_limited`, …) for UI messaging.

## Streaming chat and account routing

1. Persist the user message and an empty assistant message (`status = streaming`), then create
   a generation and first attempt.
2. Resolve eligible accounts for the conversation's fixed provider/model. The saved account is
   a preference, not a binding; disabled or credential-less accounts are skipped.
3. Register a `CancellationToken` keyed by assistant message id.
4. Build history from the last `contextMessageLimit` messages plus attachments. On retry or
   Continue, exclude the assistant row and append its committed partial text; Ollama receives
   an explicit continuation instruction, Anthropic can use trailing-assistant prefill.
5. Stream deltas into a shared buffer; flush content/segments at most every second or 2 KiB.
6. Retry at most once more on network, timeout, 5xx, or malformed/empty stream failures. Never
   retry 429/usage limits or 401/403 auth failures. Provider stays fixed.
7. On completion, finalize content, cumulative message tokens, per-run usage, and
   `status = complete`. Each attempt records its own segment and token counts.
8. On exhausted retry or explicit Continue failure, preserve partial content and mark the
   generation `interrupted` with a Continue action.
9. On app start, `recover_interrupted_generations` marks orphaned attempts, segments, and
   messages interrupted so the UI never hangs.

## Durable tasks

1. Resolve target provider/account/model through the shared account registry, then persist it
   on the task.
2. Ask the model for a JSON plan; accept 1..`taskMaxSteps` steps.
3. Execute steps sequentially with non-streaming calls, writing step output and a checkpoint
   JSON per step.
4. Pause sets a pause row and cancels the active call; resume clears it and continues from
   the first unfinished step. Cancel is terminal.
5. On success, write a Markdown artifact under the app data directory with a SHA-256.
6. On startup, `running`/`queued` tasks become `paused` and running steps become `queued`.

## Security boundaries

- Attachment import validates size/type, stores copies under app-managed paths, and computes
  SHA-256 before insert; failures roll back the row.
- Preview/open commands resolve paths against app-managed directories and refuse escapes.
- Logs go to the app log dir; secrets and full prompts are never logged.
- No cookie reading, session hijacking, account rotation, or auth bypass. Anthropic access is
  API-key only. Automatic retry never switches provider or circumvents rate/auth limits.

## Frontend structure

```
src/
  components/  Sidebar, TopBar, Composer, MessageList, Markdown, ModelPicker, Toasts
  views/       ChatView, ProjectsView, ProjectDetail, TasksView, ProvidersView,
               SettingsView, Onboarding
  stores/      ui (navigation/toasts), app (settings/init), providers, projects,
               chat (streaming/search), tasks (polling)
  lib/         api.ts (invoke wrappers), format.ts (pure helpers)
  styles/      index.css (design tokens + light/dark)
```

There is no router or component library. Navigation is a discriminated union in the `ui`
store; styling is a custom CSS design system.
