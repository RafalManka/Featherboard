CREATE TABLE checkouts_attempts
(
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    org_id          INTEGER NOT NULL REFERENCES organizations (id),
    idempotency_key TEXT    NOT NULL UNIQUE,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_checkouts_attempts_org_id ON checkouts_attempts (org_id);
