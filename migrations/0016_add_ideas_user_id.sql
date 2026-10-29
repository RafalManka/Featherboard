ALTER TABLE ideas
    ADD COLUMN user_id INTEGER REFERENCES users (id);

ALTER TABLE votes
    ADD COLUMN user_id INTEGER REFERENCES users (id);
-- should I also remove current voter_id column?