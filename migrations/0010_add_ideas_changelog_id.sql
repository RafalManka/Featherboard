ALTER TABLE ideas ADD COLUMN changelog_id INTEGER REFERENCES changelogs (id) ON DELETE CASCADE;

CREATE INDEX idx_ideas_changelog_id ON ideas (changelog_id);