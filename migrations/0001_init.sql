PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS proposals (
  id TEXT PRIMARY KEY,
  client_token TEXT UNIQUE NOT NULL,
  client_token_hash TEXT UNIQUE NOT NULL,
  manage_token_hash TEXT UNIQUE NOT NULL,
  title TEXT NOT NULL,
  freelancer_name TEXT NOT NULL,
  freelancer_email TEXT NOT NULL,
  client_name TEXT NOT NULL,
  client_email TEXT NOT NULL,
  message TEXT NOT NULL,
  currency TEXT NOT NULL,
  items_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  decision_kind TEXT,
  respondent_name TEXT,
  respondent_email TEXT,
  decision_note TEXT,
  decided_at TEXT,
  receipt_hash TEXT
);

CREATE TABLE IF NOT EXISTS deliveries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  proposal_id TEXT NOT NULL REFERENCES proposals(id) ON DELETE CASCADE,
  recipient TEXT NOT NULL,
  status TEXT NOT NULL,
  subject TEXT NOT NULL,
  body TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS receipt_tombstones (
  receipt_hash TEXT PRIMARY KEY,
  proposal_id TEXT NOT NULL,
  decided_at TEXT NOT NULL,
  deleted_at TEXT NOT NULL
);

-- Demo workspaces deliberately live outside the proposal/outbox tables. They
-- are capability-scoped, expire after 24 hours, and never create mail jobs.
CREATE TABLE IF NOT EXISTS demo_sessions (
  id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  decision_kind TEXT,
  respondent_name TEXT,
  respondent_email TEXT,
  decision_note TEXT,
  decided_at TEXT,
  receipt_hash TEXT
);

CREATE INDEX IF NOT EXISTS proposals_client_token ON proposals(client_token_hash);
CREATE INDEX IF NOT EXISTS proposals_manage_token ON proposals(manage_token_hash);
CREATE INDEX IF NOT EXISTS deliveries_proposal ON deliveries(proposal_id);
CREATE INDEX IF NOT EXISTS demo_sessions_expires ON demo_sessions(expires_at);
