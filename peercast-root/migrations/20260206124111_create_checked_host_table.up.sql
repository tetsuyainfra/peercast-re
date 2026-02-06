-- Add up migration script here
CREATE TABLE checked_hosts(
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO checked_hosts (id, name) values
    (1, "abc"),
    (2, "abcd"),
    (3, "abcefg") ;

