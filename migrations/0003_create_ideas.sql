-- 0003_create_ideas.sql  (status included from day one so Step 5 needs no migration)
CREATE TABLE ideas
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    org_id      INTEGER NOT NULL REFERENCES organizations (id),
    title       TEXT    NOT NULL,
    description TEXT    NOT NULL,
    status      TEXT    NOT NULL DEFAULT 'open'
        CHECK (status IN ('open', 'planned', 'in_progress', 'done', 'declined')),
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_ideas_org_id ON ideas (org_id);
CREATE INDEX idx_ideas_status ON ideas (status);