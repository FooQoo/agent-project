CREATE TABLE plan_revisions (
    id INTEGER PRIMARY KEY,
    work_id INTEGER NOT NULL REFERENCES work_items (id),
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    document_hash TEXT NOT NULL,
    summary TEXT NOT NULL CHECK (trim(summary) <> ''),
    UNIQUE (work_id, revision_number),
    FOREIGN KEY (work_id, document_hash) REFERENCES work_documents (work_id, hash)
);

CREATE TABLE execution_contracts (
    id INTEGER PRIMARY KEY,
    work_id INTEGER NOT NULL REFERENCES work_items (id),
    kind TEXT NOT NULL CHECK (kind IN ('plan_approval', 'direct_execution')),
    revision_id INTEGER REFERENCES plan_revisions (id),
    document_hash TEXT NOT NULL,
    authorized_by TEXT NOT NULL CHECK (trim(authorized_by) <> ''),
    authorized_at INTEGER NOT NULL,
    CHECK (
        (kind = 'plan_approval' AND revision_id IS NOT NULL) OR
        (kind = 'direct_execution' AND revision_id IS NULL)
    ),
    FOREIGN KEY (work_id, document_hash) REFERENCES work_documents (work_id, hash)
);
CREATE INDEX execution_contracts_by_work ON execution_contracts (work_id, id);

CREATE TRIGGER plan_revisions_immutable_update BEFORE UPDATE ON plan_revisions
BEGIN
    SELECT RAISE(ABORT, 'plan revisions are immutable');
END;
CREATE TRIGGER plan_revisions_immutable_delete BEFORE DELETE ON plan_revisions
BEGIN
    SELECT RAISE(ABORT, 'plan revisions cannot be deleted');
END;
CREATE TRIGGER execution_contracts_immutable_update BEFORE UPDATE ON execution_contracts
BEGIN
    SELECT RAISE(ABORT, 'execution contracts are immutable');
END;
CREATE TRIGGER execution_contracts_immutable_delete BEFORE DELETE ON execution_contracts
BEGIN
    SELECT RAISE(ABORT, 'execution contracts cannot be deleted');
END;

ALTER TABLE work_items ADD COLUMN current_revision_id INTEGER REFERENCES plan_revisions (id);
ALTER TABLE work_items ADD COLUMN current_contract_id INTEGER REFERENCES execution_contracts (id);
ALTER TABLE attempts ADD COLUMN contract_id INTEGER REFERENCES execution_contracts (id);

CREATE TABLE change_requests (
    id INTEGER PRIMARY KEY,
    work_id INTEGER NOT NULL REFERENCES work_items (id),
    body TEXT NOT NULL CHECK (trim(body) <> ''),
    state TEXT NOT NULL DEFAULT 'pending'
        CHECK (state IN ('pending', 'absorbed', 'replanned', 'rejected')),
    received_at INTEGER NOT NULL,
    resolution_reason TEXT,
    resolved_revision_id INTEGER REFERENCES plan_revisions (id)
);
CREATE INDEX change_requests_by_work ON change_requests (work_id, state, id);

CREATE TABLE completion_evidence (
    id INTEGER PRIMARY KEY,
    work_id INTEGER NOT NULL REFERENCES work_items (id),
    contract_id INTEGER NOT NULL REFERENCES execution_contracts (id),
    kind TEXT NOT NULL CHECK (kind IN ('verification', 'final_result')),
    summary TEXT NOT NULL CHECK (trim(summary) <> ''),
    document_hash TEXT,
    recorded_at INTEGER NOT NULL,
    FOREIGN KEY (work_id, document_hash) REFERENCES work_documents (work_id, hash)
);
CREATE INDEX completion_evidence_by_contract ON completion_evidence (work_id, contract_id, kind, id);
