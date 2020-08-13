INSERT INTO
    members (user_id, team_id)
VALUES
    ($1, $2)
ON CONFLICT(user_id, team_id)
    DO NOTHING
