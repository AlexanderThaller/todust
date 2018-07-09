CREATE TABLE IF NOT EXISTS
v1_entries (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  project_name  TEXT,
  started       TEXT NOT NULL,
  finished      TEXT,
  uuid          TEXT NOT NULL,
  text          TEXT
);
