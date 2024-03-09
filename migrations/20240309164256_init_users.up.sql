CREATE TABLE users (
    id VARCHAR PRIMARY KEY,
    name VARCHAR NOT NULL,
    email VARCHAR NOT NULL UNIQUE,
    picture VARCHAR NOT NULL
);

CREATE INDEX users_email_index
ON users (email);
