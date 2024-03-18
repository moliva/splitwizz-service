CREATE TYPE user_status AS ENUM ('invited', 'active', 'inactive');

CREATE TABLE users (
    id VARCHAR PRIMARY KEY,
    email VARCHAR NOT NULL UNIQUE,
    status user_status NOT NULL,
    name VARCHAR,
    picture VARCHAR,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX users_email_index
ON users (email);
