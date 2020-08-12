//! A user in the system

use crate::SqlConn;
use futures::TryStreamExt;

macro_rules! extract_user_id {
    ($user:expr) => {
        $user
            .trim_matches(|c| c == '<' || c == '>' || c == '@')
            .split('|')
            .next()
    };
}

pub struct User {
    /// The unique identifier provided by Slack
    pub id: String,

    /// The status the user sets
    pub status: Option<String>,
}

#[allow(dead_code)]
impl User {
    /// Creates a new user but does *not* save in the database
    ///
    /// # Arguments
    /// `id` - The user's Slack ID
    pub fn new(id: String) -> Self {
        // Parse the id, if necessary
        let id = extract_user_id!(id).unwrap().to_string();

        User { id, status: None }
    }

    /// Attempts to fetch a user and their status from the database, returning
    /// `None` if the user does not exist
    ///
    /// # Arguments
    /// * `db` - Connection to the SQL database
    /// * `user_id` - Slack ID of user to fetch
    pub async fn fetch(db: &mut SqlConn, user_id: &str) -> Option<Self> {
        // Parse the user id, if necessary
        let user_id = extract_user_id!(user_id).unwrap();

        let mut rows = sqlx::query_as!(
            User,
            "
            SELECT
                id, status
            FROM
                users
            WHERE
                id = ?1
            ",
            user_id
        )
        .fetch(&mut *db);

        rows.try_next().await.ok().flatten()
    }

    /// Attempts to fetch a user and their status from the database, creating
    /// a new user if one does not exist
    ///
    /// # Arguments
    /// * `db` - Connection to the SQL database
    /// * `user_id` - Slack ID of user to fetch
    pub async fn fetch_or_create(db: &mut SqlConn, user_id: &str) -> anyhow::Result<Self> {
        // Parse the user id, if necessary
        let user_id = extract_user_id!(user_id).unwrap();

        let user = sqlx::query_as!(
            User,
            "
            SELECT
                id, status
            FROM
                users
            WHERE
                id = ?1
            ",
            user_id
        )
        .fetch_one(&mut *db)
        .await;

        match user {
            Ok(user) => Ok(user),
            Err(sqlx::Error::RowNotFound) => {
                let user = User::new(user_id.to_owned());
                user.save(&mut *db).await?;
                Ok(user)
            }
            Err(e) => Err(e)?,
        }
    }

    /// Sets the user's status.
    ///
    /// This does *not* save the status in the database. To do that, you must all the `save()`
    /// funcntion.
    ///
    /// # Arguments
    /// * `status` - The user's new status
    pub fn set_status(&mut self, status: String) {
        self.status = Some(status);
    }

    /// Saves this user and their status into the database
    ///
    /// If a row for this user does not exist, then one is inserted.
    /// If one does exist, the status is updated.
    ///
    /// # Arguments
    /// * `db` - Connection to the SQL database
    pub async fn save(&self, db: &mut SqlConn) -> anyhow::Result<()> {
        // SQLx 0.4 doesn't allow refs like 0.3.5
        let id = self.id.clone();
        let status = self.status.clone();

        sqlx::query!(
            "
            INSERT INTO
                users (id, status)
            VALUES
                (?1, ?2)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status
            ",
            id,
            status
        )
        .execute(&mut *db)
        .await?;

        Ok(())
    }
}
