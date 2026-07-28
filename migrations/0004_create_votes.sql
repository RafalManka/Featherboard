-- 0004_create_votes.sql
CREATE TABLE votes
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    idea_id    INTEGER NOT NULL REFERENCES ideas (id) ON DELETE CASCADE,
    voter_id   TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (idea_id, voter_id)
);
CREATE INDEX idx_votes_idea_id ON votes (idea_id);