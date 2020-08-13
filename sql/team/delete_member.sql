DELETE FROM
    members
WHERE
    user_id = $1
        AND
    team_id = $2
