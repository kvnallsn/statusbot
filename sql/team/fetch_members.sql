SELECT
    members.user_id AS id,
    users.status
FROM
    teams
INNER JOIN
    members
    ON members.team_id = teams.id
INNER JOIN
    users
    ON users.id = members.user_id
WHERE
    teams.name = $1
