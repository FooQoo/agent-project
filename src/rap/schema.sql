CREATE TABLE work_items (
    id INTEGER PRIMARY KEY,
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    workspace TEXT NOT NULL CHECK (workspace <> ''),
    kind TEXT NOT NULL CHECK (kind IN ('epic', 'task')),
    parent_epic_id INTEGER,
    title TEXT NOT NULL CHECK (trim(title) <> ''),
    rank INTEGER NOT NULL,
    lane TEXT NOT NULL DEFAULT 'backlog',
    phase TEXT NOT NULL DEFAULT 'requested',
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (id, project_id, workspace),
    FOREIGN KEY (parent_epic_id, project_id, workspace)
        REFERENCES work_items (id, project_id, workspace),
    CHECK (parent_epic_id IS NULL OR kind = 'task'),
    CHECK (
        (lane = 'backlog' AND phase IN ('requested', 'planning', 'awaiting_approval')) OR
        (lane = 'todo' AND phase = 'ready') OR
        (lane = 'doing' AND phase IN ('ready', 'executing', 'revising', 'verifying')) OR
        (lane = 'done' AND phase = 'completed') OR
        (lane = 'canceled' AND phase = 'canceled')
    )
);
CREATE INDEX work_items_board_order ON work_items (project_id, rank, id);
CREATE INDEX work_items_epic_children ON work_items (parent_epic_id, rank, id);
CREATE TRIGGER work_items_parent_must_be_epic BEFORE INSERT ON work_items
WHEN NEW.parent_epic_id IS NOT NULL
    AND NOT EXISTS (SELECT 1 FROM work_items WHERE id = NEW.parent_epic_id AND kind = 'epic')
BEGIN
    SELECT RAISE(ABORT, 'parent must be an epic');
END;

CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    work_id INTEGER NOT NULL REFERENCES work_items (id),
    phase TEXT NOT NULL CHECK (phase IN ('planning', 'execution')),
    provider TEXT NOT NULL CHECK (trim(provider) <> ''),
    native_id TEXT
);
CREATE INDEX sessions_by_work ON sessions (work_id, id);

CREATE TABLE attempts (
    id INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL REFERENCES sessions (id),
    epoch INTEGER NOT NULL CHECK (epoch > 0),
    state TEXT NOT NULL CHECK (state IN (
        'queued', 'starting', 'running', 'stopping', 'suspended',
        'retry_wait', 'finished', 'failed', 'unknown'
    )),
    owner_token TEXT NOT NULL CHECK (trim(owner_token) <> ''),
    UNIQUE (session_id, epoch)
);
