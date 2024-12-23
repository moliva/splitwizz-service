CREATE TABLE refresh_tokens (
    id serial PRIMARY KEY,
    user_id varchar NOT NULL,
    token text NOT NULL,
    is_revoked boolean DEFAULT FALSE,
    -- device
    device_name text,
    user_agent text NOT NULL,
    device_id text NOT NULL,
    -- dates
    expires_at timestamp NOT NULL,
    created_at timestamp DEFAULT NOW(),
    -- keys
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE INDEX refresh_tokens_token_index ON refresh_tokens (token);

CREATE INDEX refresh_tokens_user_id_device_id_is_revoked_false_index ON refresh_tokens (user_id, device_id, is_revoked)
WHERE
    is_revoked = FALSE;

