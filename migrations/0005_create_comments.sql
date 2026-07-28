-- 0005_create_comments.sql  (2-level threading: parent_comment_id enforced at app layer, not CHECK)
CREATE TABLE comments
(
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    idea_id           INTEGER NOT NULL REFERENCES ideas (id) ON DELETE CASCADE,
    parent_comment_id INTEGER REFERENCES comments (id) ON DELETE CASCADE,
    author_name       TEXT    NOT NULL,
    body              TEXT    NOT NULL,
    created_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_comments_idea_id ON comments (idea_id);