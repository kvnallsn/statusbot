INSERT INTO
    members (user_id, team_id)
VALUES
    (?, ?)
ON CONFLICT(user_id, team_id)
    DO NOTHING
