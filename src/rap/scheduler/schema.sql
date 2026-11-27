CREATE TABLE IF NOT EXISTS scheduler_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    auto_run INTEGER NOT NULL CHECK (auto_run IN (0, 1)),
    max_sessions INTEGER NOT NULL CHECK (max_sessions > 0)
) STRICT;
INSERT OR IGNORE INTO scheduler_settings VALUES (1, 0, 1);

CREATE TABLE IF NOT EXISTS scheduler_work (
    work_id INTEGER PRIMARY KEY REFERENCES work_items(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('codex', 'claude_code')),
    planning_authorized INTEGER NOT NULL DEFAULT 0 CHECK (planning_authorized IN (0, 1)),
    waiting_reason TEXT,
    retry_at INTEGER,
    retry_count INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0)
) STRICT;

-- A workspace remains reserved by its work while an execution safely waits
-- for input, even though the paused attempt does not use a session slot.
CREATE TABLE IF NOT EXISTS scheduler_workspaces (
    work_id INTEGER PRIMARY KEY REFERENCES work_items(id),
    path TEXT NOT NULL UNIQUE,
    fingerprint TEXT NOT NULL
) STRICT;

-- A reservation is committed before any agent can start. Starting is a
-- separate durable step so recovery can distinguish an unused reservation
-- from an attempt for which a child process may already exist.
CREATE TABLE IF NOT EXISTS scheduler_dispatches (
    attempt_id INTEGER PRIMARY KEY REFERENCES attempts(id),
    work_id INTEGER NOT NULL REFERENCES work_items(id),
    state TEXT NOT NULL CHECK (state IN ('reserved', 'starting', 'running', 'stopping', 'unknown', 'released')),
    pid INTEGER CHECK (pid IS NULL OR pid > 0),
    process_fingerprint TEXT,
    CHECK ((pid IS NULL) = (process_fingerprint IS NULL))
) STRICT;
CREATE UNIQUE INDEX IF NOT EXISTS scheduler_dispatches_active_work
    ON scheduler_dispatches(work_id) WHERE state != 'released';

CREATE TABLE IF NOT EXISTS scheduler_provider_wait (
    provider TEXT PRIMARY KEY CHECK (provider IN ('codex', 'claude_code')),
    retry_at INTEGER NOT NULL
) STRICT;
