CREATE TABLE currencies (
  id SERIAL PRIMARY KEY,
  acronym VARCHAR NOT NULL,
  description VARCHAR DEFAULT '' NOT NULL
);

INSERT INTO currencies (acronym, description)
VALUES
                       ('USD',   'United States Dollar'),
                       ('EUR',   'Euro'),
                       ('ARS',   'Argentine Pesos');

CREATE INDEX currencies_id_index
ON currencies (id);
