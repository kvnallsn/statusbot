SELECT
    user_id AS id,
    status
FROM
    team_members
WHERE
    name = ?
