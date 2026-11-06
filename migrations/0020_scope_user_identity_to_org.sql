DROP INDEX idx_users_provider_identity;

CREATE UNIQUE INDEX idx_users_org_provider_identity
    ON users (org_id, provider, provider_user_id);
