-- Add OpenRouter and Google Gemini as first-class provider kinds.
-- SQLite cannot alter a CHECK constraint, so rebuild the providers table.
-- The migration runner disables foreign keys around this file; no other
-- table's data changes here.

CREATE TABLE providers_new (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL CHECK (kind IN ('ollama', 'anthropic', 'openrouter', 'gemini')),
    name        TEXT NOT NULL,
    base_url    TEXT NOT NULL,
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

INSERT INTO providers_new (id, kind, name, base_url, enabled, created_at, updated_at)
SELECT id, kind, name, base_url, enabled, created_at, updated_at FROM providers;

DROP TABLE providers;

ALTER TABLE providers_new RENAME TO providers;
