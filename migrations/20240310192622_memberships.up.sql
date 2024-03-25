CREATE TYPE membership_status AS ENUM ('pending', 'joined', 'rejected');

CREATE TABLE memberships (
  user_id VARCHAR NOT NULL,
  group_id SERIAL NOT NULL,

  created_by_id VARCHAR NOT NULL,
  status membership_status DEFAULT 'pending' NOT NULL,
  status_updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL, 

  FOREIGN KEY (user_id) REFERENCES users(id),
  FOREIGN KEY (group_id) REFERENCES groups(id),
  FOREIGN KEY (created_by_id) REFERENCES users(id),
  PRIMARY KEY (user_id, group_id)
);

CREATE INDEX memberships_user_id_index
ON memberships (user_id);

CREATE INDEX memberships_group_id_index
ON memberships (group_id);
