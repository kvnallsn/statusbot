INSERT INTO
    users (id, status)
VALUES
    (?, ?)
ON CONFLICT(id)
    DO UPDATE SET
        status = excluded.status
