-- Mouchak Mail initial schema migration
-- Create projects table
CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    slug TEXT NOT NULL UNIQUE,
    human_key TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create products table
CREATE TABLE IF NOT EXISTS products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_uid TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL UNIQUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create product_project_links table
CREATE TABLE IF NOT EXISTS product_project_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL,
    project_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (product_id) REFERENCES products(id),
    FOREIGN KEY (project_id) REFERENCES projects(id),
    UNIQUE (product_id, project_id)
);

-- Create agents table
CREATE TABLE IF NOT EXISTS agents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    program TEXT NOT NULL,
    model TEXT NOT NULL,
    task_description TEXT NOT NULL DEFAULT '',
    inception_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_active_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    attachments_policy TEXT NOT NULL DEFAULT 'auto',
    contact_policy TEXT NOT NULL DEFAULT 'auto',
    FOREIGN KEY (project_id) REFERENCES projects(id),
    UNIQUE (project_id, name)
);

-- Create messages table
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    sender_id INTEGER NOT NULL,
    thread_id TEXT,
    subject TEXT NOT NULL,
    body_md TEXT NOT NULL,
    importance TEXT NOT NULL DEFAULT 'normal',
    ack_required BOOLEAN NOT NULL DEFAULT FALSE,
    created_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    attachments JSON NOT NULL DEFAULT '[]',
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (sender_id) REFERENCES agents(id)
);

-- Create an FTS5 table for message bodies
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    body_md,
    content='messages',
    content_rowid='id'
);

-- Trigger to keep messages_fts updated with messages table
CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
  INSERT INTO messages_fts(rowid, body_md) VALUES (new.id, new.body_md);
END;

CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
  INSERT INTO messages_fts(messages_fts, rowid, body_md) VALUES('delete', old.id, old.body_md);
END;

CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
  INSERT INTO messages_fts(messages_fts, rowid, body_md) VALUES('delete', old.id, old.body_md);
  INSERT INTO messages_fts(rowid, body_md) VALUES (new.id, new.body_md);
END;

-- Create message_recipients table
CREATE TABLE IF NOT EXISTS message_recipients (
    message_id INTEGER NOT NULL,
    agent_id INTEGER NOT NULL,
    recipient_type TEXT NOT NULL DEFAULT 'to',
    read_ts DATETIME,
    ack_ts DATETIME,
    PRIMARY KEY (message_id, agent_id),
    FOREIGN KEY (message_id) REFERENCES messages(id),
    FOREIGN KEY (agent_id) REFERENCES agents(id),
    CHECK (recipient_type IN ('to', 'cc', 'bcc'))
);

-- Create file_reservations table
CREATE TABLE IF NOT EXISTS file_reservations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    agent_id INTEGER NOT NULL,
    path_pattern TEXT NOT NULL,
    exclusive BOOLEAN NOT NULL DEFAULT TRUE,
    reason TEXT NOT NULL DEFAULT '',
    created_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_ts DATETIME NOT NULL,
    released_ts DATETIME,
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (agent_id) REFERENCES agents(id)
);

-- Create agent_links table
CREATE TABLE IF NOT EXISTS agent_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    a_project_id INTEGER NOT NULL,
    a_agent_id INTEGER NOT NULL,
    b_project_id INTEGER NOT NULL,
    b_agent_id INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    reason TEXT NOT NULL DEFAULT '',
    created_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_ts DATETIME,
    FOREIGN KEY (a_project_id) REFERENCES projects(id),
    FOREIGN KEY (a_agent_id) REFERENCES agents(id),
    FOREIGN KEY (b_project_id) REFERENCES projects(id),
    FOREIGN KEY (b_agent_id) REFERENCES agents(id),
    UNIQUE (a_project_id, a_agent_id, b_project_id, b_agent_id)
);

-- Create project_sibling_suggestions table
CREATE TABLE IF NOT EXISTS project_sibling_suggestions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_a_id INTEGER NOT NULL,
    project_b_id INTEGER NOT NULL,
    score REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'suggested',
    rationale TEXT NOT NULL DEFAULT '',
    created_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    evaluated_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    confirmed_ts DATETIME,
    dismissed_ts DATETIME,
    FOREIGN KEY (project_a_id) REFERENCES projects(id),
    FOREIGN KEY (project_b_id) REFERENCES projects(id),
    UNIQUE (project_a_id, project_b_id)
);

-- Create build_slots table
CREATE TABLE IF NOT EXISTS build_slots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    agent_id INTEGER NOT NULL,
    slot_name TEXT NOT NULL,
    created_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_ts DATETIME NOT NULL,
    released_ts DATETIME,
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (agent_id) REFERENCES agents(id)
);

-- Create macros table
CREATE TABLE IF NOT EXISTS macros (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    steps JSON NOT NULL DEFAULT '[]',
    created_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id),
    UNIQUE (project_id, name)
);

-- Create overseer_messages table
CREATE TABLE IF NOT EXISTS overseer_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    sender_id INTEGER NOT NULL,
    subject TEXT NOT NULL,
    body_md TEXT NOT NULL,
    importance TEXT NOT NULL DEFAULT 'normal',
    created_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    read_ts DATETIME,
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (sender_id) REFERENCES agents(id)
);
