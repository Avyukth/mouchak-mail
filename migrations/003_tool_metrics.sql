-- Tool Metrics table (idempotent migration)
CREATE TABLE IF NOT EXISTS tool_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER, -- Nullable (e.g. error before project resolved)
    agent_id INTEGER,   -- Nullable (e.g. project-level tool)
    tool_name TEXT NOT NULL,
    args_json TEXT, -- Optional JSON string of arguments (could be truncated)
    status TEXT NOT NULL, -- 'success' or 'error'
    error_code TEXT,      -- Optional error code if status is error
    duration_ms INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tool_metrics_project_created ON tool_metrics(project_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_tool_metrics_tool_name ON tool_metrics(tool_name);
