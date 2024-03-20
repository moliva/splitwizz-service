CREATE TABLE expenses (
  id SERIAL NOT NULL PRIMARY KEY,
  group_id SERIAL NOT NULL,
  payer_id VARCHAR NOT NULL,

  description VARCHAR DEFAULT '' NOT NULL,
  currency_id SERIAL NOT NULL,
  amount DECIMAL NOT NULL,
  date TIMESTAMP WITH TIME ZONE NOT NULL, 
  split_strategy JSONB NOT NULL,

  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL, 
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL, 

  FOREIGN KEY (payer_id) REFERENCES users(id),
  FOREIGN KEY (group_id) REFERENCES groups(id),
  FOREIGN KEY (currency_id) REFERENCES currencies(id)
);

CREATE INDEX expenses_group_id_index
ON expenses (group_id);

CREATE INDEX expenses_payer_id_index
ON expenses (payer_id);

CREATE INDEX expenses_date_index
ON expenses (date);
