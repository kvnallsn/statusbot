INSERT INTO
    users (id, status)
VALUES
    ($1, $2)
ON CONFLICT(id)
    DO UPDATE SET
        status = excluded.status
