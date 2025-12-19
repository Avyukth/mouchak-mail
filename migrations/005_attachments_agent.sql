-- Add agent association to attachments (idempotent migration)
-- Allows filtering attachments by the agent that created them

-- Add agent_id column (nullable FK to agents)
ALTER TABLE attachments ADD COLUMN agent_id INTEGER REFERENCES agents(id);

-- Add index for agent filtering
CREATE INDEX IF NOT EXISTS idx_attachments_agent ON attachments(agent_id);

-- Add composite index for project+agent filtering
CREATE INDEX IF NOT EXISTS idx_attachments_project_agent ON attachments(project_id, agent_id);
