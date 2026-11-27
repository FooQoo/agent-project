CREATE TABLE rap_results (
    id INTEGER PRIMARY KEY,
    work_id INTEGER NOT NULL REFERENCES work_items (id),
    contract_id INTEGER NOT NULL REFERENCES execution_contracts (id),
    revision_id INTEGER REFERENCES plan_revisions (id),
    stage TEXT NOT NULL CHECK (stage IN ('interim', 'final')),
    title TEXT NOT NULL CHECK (trim(title) <> ''),
    summary TEXT NOT NULL CHECK (trim(summary) <> ''),
    links_json TEXT NOT NULL DEFAULT '[]',
    document_hash TEXT,
    media_type TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (work_id, document_hash) REFERENCES work_documents (work_id, hash)
);
CREATE INDEX rap_results_by_work ON rap_results (work_id, id);
