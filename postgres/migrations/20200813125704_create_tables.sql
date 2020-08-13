-- Create all tables for v0.1.0
CREATE TABLE IF NOT EXISTS users (
    id          TEXT NOT NULL PRIMARY KEY,
    status      TEXT
);

CREATE TABLE IF NOT EXISTS teams (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT NOT NULL 
);

CREATE UNIQUE INDEX IF NOT EXISTS
        idx_teams_name
    ON
        teams(name);

CREATE TABLE IF NOT EXISTS members (
    user_id     TEXT NOT NULL,
    team_id     BIGINT NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id),
    FOREIGN KEY(team_id) REFERENCES teams(id),
    UNIQUE(user_id, team_id)
);
