CREATE TABLE agent_capabilities (
    id INTEGER PRIMARY KEY,
    agent_id INTEGER REFERENCES agents(id),
    capability TEXT NOT NULL,
    granted_at TEXT NOT NULL
);
CREATE INDEX idx_agent_capabilities_agent_id ON agent_capabilities(agent_id);
