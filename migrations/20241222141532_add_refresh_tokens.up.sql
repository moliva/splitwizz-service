CREATE TABLE refresh_tokens (
    id serial PRIMARY KEY,
    user_id varchar NOT NULL,
    token text NOT NULL,
    expires_at timestamp NOT NULL,
    is_revoked boolean DEFAULT FALSE,
    -- keys
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE INDEX refresh_tokens_token_index ON refresh_tokens (token);

