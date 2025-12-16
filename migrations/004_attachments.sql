-- Attachments table (idempotent migration)
CREATE TABLE IF NOT EXISTS attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    filename TEXT NOT NULL,
    stored_path TEXT NOT NULL,
    media_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    created_ts TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_attachments_project ON attachments(project_id);
