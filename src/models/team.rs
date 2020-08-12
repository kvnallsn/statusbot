//! Team Representation for sqlx

use crate::{models::User, SqlConn};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Team {
    // unique team id
    id: i64,

    // Name of team
    pub name: String,
}

#[allow(dead_code)]
impl Team {
    /// Creates a new team with the supplied name and save
    /// it in the database
    ///
    /// # Arguments
    /// * `name` - Name of this team
    pub async fn new(db: &mut SqlConn, name: &str) -> anyhow::Result<Self> {
        sqlx::query!(
            "
            INSERT INTO
                teams (name)
            VALUES
                (?1)
            ",
            name
        )
        .execute(&mut *db)
        .await?;

        let team = sqlx::query_as!(
            Team,
            "
            SELECT
                id, name
            FROM
                teams
            WHERE
                name = ?1
            ",
            name
        )
        .fetch_one(&mut *db)
        .await?;

        Ok(team)
    }

    /// Attempts to retrieve a team from the database, returning None if one does not exist
    ///
    /// # Arguments
    /// * `db` - Connection to SQL database
    /// * `name` - Name of team to fetch
    pub async fn fetch(db: &mut SqlConn, name: &str) -> Option<Self> {
        let mut row = sqlx::query_as!(
            Team,
            "
            SELECT
                id, name
            FROM
                teams
            WHERE
                name = ?1
            ",
            name
        )
        .fetch(&mut *db);

        row.try_next().await.ok().flatten()
    }

    /// Fetches all teams from the database
    ///
    /// # Arguments
    /// * `db` - Conenction to the SQL database
    pub async fn fetch_all(db: &mut SqlConn) -> anyhow::Result<Vec<Team>> {
        let teams = sqlx::query_as!(
            Team,
            "
            SELECT
                id, name
            FROM
                teams
            "
        )
        .fetch_all(&mut *db)
        .await?;

        Ok(teams)
    }

    /// Returns all members belonging to a team with name `name`
    ///
    /// # Arguments
    /// * `db` - Connection to SQL database
    /// * `team_name` - Name of this team
    pub async fn members(db: &mut SqlConn, team_name: &str) -> anyhow::Result<Vec<User>> {
        let users = sqlx::query_as!(
            User,
            "
            SELECT
                user_id AS id, status
            FROM
                team_members
            WHERE
                name = ?1
            ",
            team_name
        )
        .fetch_all(&mut *db)
        .await?;

        Ok(users)
    }

    /// Adds a member to this team.
    ///
    /// If the member is already on this team, do nothing
    ///
    /// # Arguments
    /// * `db` - Conenction to SQL database
    /// * `user` - User to add
    pub async fn add_member(&self, db: &mut SqlConn, user: &User) -> anyhow::Result<()> {
        sqlx::query!(
            "
            INSERT INTO
                members (user_id, team_id)
            VALUES
                (?1, ?2)
            ON CONFLICT(user_id, team_id) DO NOTHING
            ",
            user.id,
            self.id
        )
        .execute(&mut *db)
        .await?;

        Ok(())
    }

    /// Deletes a member from the team.
    ///
    /// If the member isn't a part of the team, does nothing.
    ///
    /// # Arguments
    /// * `db` - Conenction to SQL database
    /// * `user` - User to add
    pub async fn delete_member(&self, db: &mut SqlConn, user: &User) -> anyhow::Result<()> {
        sqlx::query!(
            "
            DELETE FROM
                members
            WHERE
                user_id = ?1
                    AND
                team_id = ?2
            ",
            user.id,
            self.id
        )
        .execute(&mut *db)
        .await?;

        Ok(())
    }

    /// Saves this team into the database
    ///
    /// If this team does not exist, a new record is created.  If it does,
    /// the team name is updated
    ///
    /// # Arguments
    /// * `db` - Connection to SQL database
    pub async fn save(&self, db: &mut SqlConn) -> anyhow::Result<()> {
        sqlx::query!(
            "
            UPDATE
                teams
            SET
                name = ?1
            WHERE
                id = ?2
            ",
            self.name,
            self.id
        )
        .execute(&mut *db)
        .await?;

        Ok(())
    }

    /// Deletes this team from the database
    ///
    /// *THIS ACTION CANNOT BE UNDONE*
    pub async fn delete(self, db: &mut SqlConn) -> anyhow::Result<()> {
        sqlx::query!(
            "
            DELETE FROM
                teams
            WHERE
                id = ?1
            ",
            self.id
        )
        .execute(&mut *db)
        .await?;

        Ok(())
    }
}
