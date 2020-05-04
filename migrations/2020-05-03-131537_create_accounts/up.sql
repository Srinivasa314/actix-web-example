CREATE TABLE accounts (
  username VARCHAR(255) PRIMARY KEY,
  password_hash BINARY(32) NOT NULL
)