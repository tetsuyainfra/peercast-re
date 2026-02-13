-- Add up migration script here
CREATE TABLE checked_hosts(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ip_address BLOB NOT NULL,
    port INTEGER NOT NULL,
    speed INTEGER DEFAULT 0, -- 0: closed, 1~: open and speed
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- CHECK (created_at GLOB '____-__-__T__:*Z')
    CHECK (length(ip_address) == 4 OR length(ip_address) == 16),-- Ensure IPv4/IPv6 length
    CHECK (port >= 0 AND port <= 65535),
    UNIQUE (ip_address, port)
    -- UNIQUE (ip, created_at)
) STRICT; -- STRICT to enforce data types

INSERT INTO checked_hosts (id, ip_address, port, speed) values
    (1, X'FF000001', 80,   0),
    (2, X'FF000002', 7144, 1),
    (3, X'FF000003', 7144, 1),
    (4, X'FF000003', 7145, 0),
    (5, X'FF000003', 7146, -1),
    (6, X'80000001', 1, 1000),
    (7, X'80000002', 2, 2000)
    ;


