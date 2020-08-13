-- Create all tables for v0.1.0
CREATE TABLE IF NOT EXISTS users (
    id          TEXT NOT NULL PRIMARY KEY,
    status      TEXT
);

CREATE TABLE IF NOT EXISTS teams (
    id          SERIAL PRIMARY KEY,
    name        TEXT NOT NULL 
);

CREATE UNIQUE INDEX IF NOT EXISTS
        idx_teams_name
    ON
        teams(name);

CREATE TABLE IF NOT EXISTS members (
    user_id     TEXT NOT NULL,
    team_id     INTEGER NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id),
    FOREIGN KEY(team_id) REFERENCES teams(id),
    UNIQUE(user_id, team_id)
);

CREATE OR REPLACE VIEW team_members
AS
    SELECT
        teams.id, teams.name, members.user_id, users.status
    FROM
        teams
    INNER JOIN
        members
        ON members.team_id = teams.id
    INNER JOIN
        users
        ON users.id = members.user_id
