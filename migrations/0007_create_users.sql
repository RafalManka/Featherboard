CREATE TABLE users
(
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    org_id           INTEGER NOT NULL REFERENCES organizations (id),
    provider         TEXT    NOT NULL,
    provider_user_id TEXT    NOT NULL,
    email            TEXT    NOT NULL,
    name             TEXT    NOT NULL,
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_users_provider_identity ON users (provider, provider_user_id);