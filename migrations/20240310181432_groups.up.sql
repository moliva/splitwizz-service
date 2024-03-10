CREATE TABLE groups (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    creator_id VARCHAR NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (creator_id) REFERENCES users(id)
);

CREATE INDEX groups_id_index
ON groups (id);
