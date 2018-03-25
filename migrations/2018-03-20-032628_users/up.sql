CREATE TABLE users (
  access_id BIGINT NOT NULL,
  username TEXT UNIQUE NOT NULL,
  buying INTEGER NOT NULL,
  selling INTEGER NOT NULL,
  confirmed INTEGER NOT NULL,
  created TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY ( access_id, username)
);

CREATE INDEX created_idx ON users(created);
