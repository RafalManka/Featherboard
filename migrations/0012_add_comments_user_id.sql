ALTER TABLE comments
    ADD COLUMN user_id INTEGER REFERENCES users (id);