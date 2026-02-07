-- Add up migration script here
CREATE TABLE checked_hosts(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ip BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    -- CHECK (created_at GLOB '____-__-__T__:*Z')
    CHECK (length(ip) == 4 OR length(ip) == 16),-- Ensure IPv4/IPv6 length
    UNIQUE (ip)
    -- UNIQUE (ip, created_at)
) STRICT; -- STRICT to enforce data types

INSERT INTO checked_hosts (id, ip) values
    (1, X'FF000001'),
    (2, X'FF000002'),
    (3, X'FF000003')
    ;


