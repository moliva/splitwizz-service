CREATE TABLE GROUPS (
    id serial PRIMARY KEY,
    -- actual data
    name varchar NOT NULL,
    creator_id varchar NOT NULL,
    default_currency_id serial NOT NULL,
    balance_config jsonb NOT NULL,
    -- date metadata
    created_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- keys
    FOREIGN KEY (creator_id) REFERENCES users (id),
    FOREIGN KEY (default_currency_id) REFERENCES currencies (id)
);

CREATE INDEX groups_id_index ON GROUPS (id);

