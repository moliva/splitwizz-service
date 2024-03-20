CREATE TABLE currencies (
  id SERIAL PRIMARY KEY,
  description VARCHAR NOT NULL
);

CREATE INDEX currencies_id_index
ON currencies (id);
