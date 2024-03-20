CREATE TABLE currencies (
  id SERIAL PRIMARY KEY,
  acronym VARCHAR NOT NULL,
  description VARCHAR DEFAULT '' NOT NULL
);

CREATE INDEX currencies_id_index
ON currencies (id);
