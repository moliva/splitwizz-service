CREATE TABLE memberships (
  user_id VARCHAR NOT NULL,
  group_id SERIAL NOT NULL,
  status VARCHAR DEFAULT 'pending' NOT NULL,
  status_updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL, 

  FOREIGN KEY (user_id) REFERENCES users(id),
  FOREIGN KEY (group_id) REFERENCES groups(id),
  PRIMARY KEY (user_id, group_id)
);

CREATE INDEX memberships_user_id_index
ON memberships (user_id);

CREATE INDEX memberships_group_id_index
ON memberships (group_id);
