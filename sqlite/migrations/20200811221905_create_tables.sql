-- Create all tables for v0.1.0
CREATE TABLE IF NOT EXISTS users (
    id          TEXT NOT NULL PRIMARY KEY,
    status      TEXT
);

CREATE TABLE IF NOT EXISTS teams (
    id          INTEGER NOT NULL PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS members (
    user_id     TEXT NOT NULL,
    team_id     INTEGER NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id),
    FOREIGN KEY(team_id) REFERENCES teams(id),
    UNIQUE(user_id, team_id)
);
