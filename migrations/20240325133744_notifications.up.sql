CREATE TYPE notification_status AS ENUM ('new', 'read', 'archived');

CREATE TABLE notifications (
  id SERIAL PRIMARY KEY,
  user_id VARCHAR NOT NULL,
  data JSONB NOT NULL,

  status notification_status DEFAULT 'new' NOT NULL,
  status_updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL, 

  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL, 

  FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX notifications_user_id_index
ON notifications (user_id);
