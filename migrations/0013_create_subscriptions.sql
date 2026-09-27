CREATE TABLE subscriptions
(
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    org_id                 INTEGER NOT NULL REFERENCES organizations (id),
    stripe_customer_id     TEXT    NOT NULL UNIQUE,
    stripe_subscription_id TEXT    NOT NULL UNIQUE,
    status                 TEXT    NOT NULL
        CHECK (status IN
               ('incomplete', 'incomplete_expired', 'trialing', 'active', 'past_due', 'canceled', 'unpaid', 'paused')),
    created_at             TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at             TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_subscriptions_org_id ON subscriptions (org_id);
