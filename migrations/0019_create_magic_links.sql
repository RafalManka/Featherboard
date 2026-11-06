CREATE TABLE magic_links
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    org_id     INTEGER NOT NULL REFERENCES organizations (id),
    email      TEXT    NOT NULL,
    token_hash TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    expires_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+5 minutes'))
);

CREATE UNIQUE INDEX idx_magic_links_email_token_hash ON magic_links (email, org_id, token_hash);