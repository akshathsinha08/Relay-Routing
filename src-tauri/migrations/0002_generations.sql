-- Account-independent conversations and resumable generations.
-- Additive only: 0001_init.sql must never be edited after release.

ALTER TABLE messages ADD COLUMN interrupted INTEGER NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN interruption_reason TEXT;

CREATE TABLE IF NOT EXISTS generations (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  user_message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  assistant_message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'streaming', 'completed', 'interrupted', 'failed', 'cancelled')),
  requested_provider_id TEXT,
  requested_account_id TEXT,
  requested_model TEXT,
  attempt_count INTEGER NOT NULL DEFAULT 0,
  error_code TEXT,
  error_message TEXT,
  started_at TEXT,
  finished_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_generations_conversation
  ON generations(conversation_id, created_at);
CREATE INDEX IF NOT EXISTS idx_generations_status
  ON generations(status);

CREATE TABLE IF NOT EXISTS generation_attempts (
  id TEXT PRIMARY KEY,
  generation_id TEXT NOT NULL REFERENCES generations(id) ON DELETE CASCADE,
  attempt_no INTEGER NOT NULL,
  provider_id TEXT NOT NULL REFERENCES providers(id),
  account_id TEXT NOT NULL REFERENCES accounts(id),
  model TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'streaming'
    CHECK (status IN ('streaming', 'completed', 'interrupted', 'failed', 'cancelled')),
  error_code TEXT,
  error_message TEXT,
  finish_reason TEXT,
  input_tokens INTEGER,
  output_tokens INTEGER,
  started_at TEXT,
  finished_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (generation_id, attempt_no)
);

CREATE INDEX IF NOT EXISTS idx_generation_attempts_generation
  ON generation_attempts(generation_id, attempt_no);

CREATE TABLE IF NOT EXISTS message_segments (
  id TEXT PRIMARY KEY,
  message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  generation_id TEXT REFERENCES generations(id) ON DELETE CASCADE,
  attempt_id TEXT REFERENCES generation_attempts(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL,
  content TEXT NOT NULL DEFAULT '',
  reasoning TEXT,
  status TEXT NOT NULL DEFAULT 'streaming'
    CHECK (status IN ('streaming', 'complete', 'interrupted', 'failed')),
  provider_id TEXT,
  account_id TEXT,
  model TEXT,
  input_tokens INTEGER,
  output_tokens INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (message_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_message_segments_message
  ON message_segments(message_id, seq);
