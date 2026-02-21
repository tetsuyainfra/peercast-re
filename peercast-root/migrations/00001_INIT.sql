-- Add up migration script here
CREATE TABLE checked_hosts(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ip_address BLOB NOT NULL,
    port INTEGER NOT NULL,
    -- port_level:  -1: ポート未開放, 0: 未チェック, 1: ポート開放, 2: ポート開放かつ速度測定済み
    port_level INTEGER NOT NULL DEFAULT 0,
    port_speed INTEGER NULL , -- ポートの速度 (Mbps), port_levelがWelldoneWithSpeedのときのみ有効 1〜1000Mbps
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- CHECK (created_at GLOB '____-__-__T__:*Z')
    UNIQUE (ip_address, port),
    CHECK (length(ip_address) == 4 OR length(ip_address) == 16),-- Ensure IPv4/IPv6 length
    CHECK (port >= 0 AND port <= 65535),
    CHECK (port_level IN (-1, 0, 1, 2)),
    CHECK (
        (port_level = 2 AND port_speed IS NOT NULL)
        OR
        (port_level != 2 AND port_speed IS NULL)
    )
    -- port_speed is NOT NULL or between 1 and 1000
    CHECK (
        port_speed IS NULL
        OR (port_speed <= 1000)
    )
) STRICT; -- STRICT to enforce data types

INSERT INTO checked_hosts (id, ip_address, port, port_level, port_speed) values
    (1, X'FF000001', 80,   1, NULL),
    (2, X'FF000002', 7144, 1, NULL),
    (3, X'FF000003', 7144, 1, NULL),
    (4, X'FF000003', 7145, 1, NULL),
    (5, X'FF000003', 7146, -1, NULL),
    (6, X'80000001', 1,    2, 2),
    (7, X'80000002', 2,    2, 2)
    ;


