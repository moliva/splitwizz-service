CREATE TABLE currencies (
    id serial PRIMARY KEY,
    acronym varchar NOT NULL,
    description varchar DEFAULT '' NOT NULL
);

INSERT INTO currencies (acronym, description)
VALUES ('USD', 'United States Dollar'),
('EUR', 'Euro'),
('ARS', 'Argentine Pesos');

CREATE INDEX currencies_id_index ON currencies (id);
