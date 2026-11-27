CREATE TABLE legacy_id_map (
    project_id TEXT NOT NULL,
    entity_kind TEXT NOT NULL CHECK (entity_kind IN ('epic', 'task', 'question', 'result', 'attachment')),
    legacy_id TEXT NOT NULL,
    rap_id INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    PRIMARY KEY (project_id, entity_kind, legacy_id)
);

CREATE TABLE legacy_history (
    id INTEGER PRIMARY KEY,
    work_id INTEGER NOT NULL REFERENCES work_items (id),
    recorded_at TEXT NOT NULL,
    reason TEXT NOT NULL
);
CREATE INDEX legacy_history_by_work ON legacy_history (work_id, id);
