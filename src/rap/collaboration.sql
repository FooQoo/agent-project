CREATE TABLE rap_questions (
    id INTEGER PRIMARY KEY,
    work_id INTEGER NOT NULL REFERENCES work_items (id),
    session_id INTEGER NOT NULL REFERENCES sessions (id),
    prompt TEXT NOT NULL CHECK (trim(prompt) <> ''),
    asked_at INTEGER NOT NULL,
    answer_body TEXT,
    answered_by TEXT,
    answered_at INTEGER,
    delivery_state TEXT NOT NULL DEFAULT 'open'
        CHECK (delivery_state IN ('open', 'pending_delivery', 'delivered')),
    delivered_at INTEGER,
    delivery_attempts INTEGER NOT NULL DEFAULT 0 CHECK (delivery_attempts >= 0),
    CHECK ((answer_body IS NULL) = (answered_at IS NULL))
);
CREATE INDEX rap_questions_by_work ON rap_questions (work_id, delivery_state, id);
