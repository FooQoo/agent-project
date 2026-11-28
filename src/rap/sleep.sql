CREATE TABLE sleep_mode (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    active INTEGER NOT NULL DEFAULT 0 CHECK (active IN (0, 1)),
    started_at INTEGER,
    ended_at INTEGER
) STRICT;
INSERT OR IGNORE INTO sleep_mode (id, active) VALUES (1, 0);

CREATE TABLE delegated_decisions (
    id INTEGER PRIMARY KEY,
    question_id INTEGER NOT NULL UNIQUE REFERENCES rap_questions (id),
    work_id INTEGER NOT NULL REFERENCES work_items (id),
    delegate_actor TEXT NOT NULL CHECK (trim(delegate_actor) <> ''),
    rationale TEXT NOT NULL CHECK (trim(rationale) <> ''),
    risk_level TEXT NOT NULL CHECK (risk_level IN ('routine', 'privilege_escalation', 'irreversible')),
    status TEXT NOT NULL CHECK (status IN ('delegated', 'held_for_human')),
    decided_at INTEGER NOT NULL
);
CREATE INDEX delegated_decisions_by_time ON delegated_decisions (decided_at, id);
