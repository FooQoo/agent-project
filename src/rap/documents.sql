CREATE TABLE work_documents (
    work_id INTEGER NOT NULL REFERENCES work_items (id),
    hash TEXT NOT NULL CHECK (length(hash) = 64 AND hash NOT GLOB '*[^0-9a-f]*'),
    PRIMARY KEY (work_id, hash)
);
CREATE INDEX work_documents_by_hash ON work_documents (hash);
