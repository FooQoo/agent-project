CREATE TABLE rap_notifications (
    id INTEGER PRIMARY KEY,
    work_id INTEGER NOT NULL REFERENCES work_items (id),
    kind TEXT NOT NULL CHECK (kind IN ('question_needed', 'safety_hold', 'result_ready', 'unrecoverable')),
    target_route TEXT NOT NULL CHECK (trim(target_route) <> ''),
    title TEXT NOT NULL CHECK (trim(title) <> ''),
    body TEXT NOT NULL CHECK (trim(body) <> ''),
    safety_critical INTEGER NOT NULL CHECK (safety_critical IN (0, 1)),
    state TEXT NOT NULL CHECK (state IN ('pending', 'batched_for_sleep', 'sent', 'failed')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error TEXT,
    created_at INTEGER NOT NULL,
    delivered_at INTEGER
);
CREATE INDEX rap_notifications_by_state ON rap_notifications (state, id);
