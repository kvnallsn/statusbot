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
        sqlx::query_file!("sql/team/insert.sql", name)
            .execute(&mut *db)
            .await?;

        let team = sqlx::query_file_as!(Team, "sql/team/fetch_by_name.sql", name)
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
        let mut row =
            sqlx::query_file_as!(Team, "sql/team/fetch_by_name.sql", name).fetch(&mut *db);

        row.try_next().await.ok().flatten()
    }

    /// Fetches all teams from the database
    ///
    /// # Arguments
    /// * `db` - Conenction to the SQL database
    pub async fn fetch_all(db: &mut SqlConn) -> anyhow::Result<Vec<Team>> {
        let teams = sqlx::query_file_as!(Team, "sql/team/fetch_all.sql")
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
        let users = sqlx::query_file_as!(User, "sql/team/fetch_members.sql", team_name)
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
        sqlx::query_file!("sql/team/add_member.sql", user.id, self.id)
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
        sqlx::query_file!("sql/team/delete_member.sql", user.id, self.id)
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
        sqlx::query_file!("sql/team/save.sql", self.name, self.id)
            .execute(&mut *db)
            .await?;

        Ok(())
    }

    /// Deletes this team from the database
    ///
    /// *THIS ACTION CANNOT BE UNDONE*
    pub async fn delete(self, db: &mut SqlConn) -> anyhow::Result<()> {
        sqlx::query_file!("sql/team/delete.sql", self.id)
            .execute(&mut *db)
            .await?;

        Ok(())
    }
}
