CREATE TABLE expenses (
    -- ids
    id serial NOT NULL PRIMARY KEY,
    group_id serial NOT NULL,
    -- status
    deleted boolean NOT NULL DEFAULT FALSE,
    -- data
    description varchar DEFAULT '' NOT NULL,
    currency_id serial NOT NULL,
    amount double precision NOT NULL,
    date timestamp with time zone NOT NULL,
    split_strategy jsonb NOT NULL,
    -- created action
    created_by_id varchar NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    -- updated action
    updated_by_id varchar NOT NULL,
    updated_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    -- keys
    FOREIGN KEY (group_id) REFERENCES GROUPS (id),
    FOREIGN KEY (currency_id) REFERENCES currencies (id),
    FOREIGN KEY (created_by_id) REFERENCES users (id),
    FOREIGN KEY (updated_by_id) REFERENCES users (id)
);

CREATE INDEX expenses_group_id_index ON expenses (group_id);

CREATE INDEX expenses_date_index ON expenses (date);

