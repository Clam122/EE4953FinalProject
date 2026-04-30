-- Your SQL goes here
CREATE TABLE users (
    id               INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    email            TEXT NOT NULL UNIQUE,
    public_key       TEXT NOT NULL UNIQUE,
    hash_verify      TEXT NOT NULL,
    visible_to_public INTEGER NOT NULL DEFAULT 0
);