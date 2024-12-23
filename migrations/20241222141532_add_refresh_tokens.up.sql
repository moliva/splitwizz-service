CREATE TABLE refresh_tokens (
    id serial PRIMARY KEY,
    user_id varchar NOT NULL,
    token text NOT NULL,
    expires_at timestamp NOT NULL DEFAULT NOW(),
    is_revoked boolean DEFAULT FALSE,
    -- keys
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE INDEX refresh_tokens_token_index ON refresh_tokens (token);

CREATE INDEX refresh_tokens_user_id_is_revoked_false_index ON refresh_tokens (user_id, is_revoked)
WHERE
    is_revoked = FALSE;

