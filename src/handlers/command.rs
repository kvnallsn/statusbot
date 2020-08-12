use crate::{
    models::{Team, User},
    HasDb, State,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::borrow::Cow;
use tide::StatusCode;

macro_rules! header {
    ($container:expr, $text:expr) => {
        $container.push(serde_json::json!({
            "type": "header",
            "text": {
                "type": "plain_text",
                "text": $text,
            }
        }))
    }
}

macro_rules! mrkdwn {
    ($container:expr, $text:expr) => {
        $container.push(serde_json::json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": $text,
            }
        }))
    }
}

macro_rules! divider {
    ($container:expr) => {
        $container.push(serde_json::json!({ "type": "divider" }))
    }
}

#[derive(Debug, Deserialize)]
struct SlashCommand {
    // Deprecated verification token (use signed secrets instead)
    pub token: String,

    /// The slash command that was typed (e.g., /location)
    pub command: String,

    /// The text following the slash command (e.g., telework)
    pub text: String,

    /// A temporary webhook that can be used to generated messages responses
    pub response_url: String,

    /// A short-lived ID that will let you open a modal
    pub trigger_id: String,

    /// The ID of the user who triggered the command
    pub user_id: String,

    /// The plain text name of the user who triggered the command
    /// *Do not rely on this field* as it is being phased out. Use
    /// the `user_id` field instead
    pub user_name: String,

    /// The ID of the team who this bot belongs to
    pub team_id: String,

    /// The ID of the channel this message was sent in
    pub channel_id: String,

    /// Your Slack app's unique identifier.  Use this in conjection with
    /// request signing to verify context for inbound requests
    pub api_app_id: String,
}

pub enum SlashAction<'a> {
    /// Shows a user's last set status
    ShowUser { user: &'a str },

    /// Shows all members on a team statuses
    ShowTeam { team: &'a str },

    /// List all teams (no members)
    ListTeams,

    /// Creates a new team
    CreateTeam { name: &'a str },

    /// Deletes an existing team
    DeleteTeam { name: &'a str },

    /// Adds a memeber to an existing team
    AddMember { team: &'a str, user: &'a str },

    /// Removes a member from an existing team
    RemoveMember { team: &'a str, user: &'a str },

    /// A specific error message is parsing failed
    ParsingFailed(Cow<'a, str>),
}

impl<'a> SlashAction<'a> {
    /// Parses a received command line into a `SlashAAction`
    ///
    /// # Arguments
    /// * `text` - Text received from `SlashCommand`
    ///
    /// # Examples
    /// ```rust
    /// let action = SlashAction::parse("team create Senate");
    /// assert_eq!(action, SlashAction::CreateTeam { team: "Senate" });
    /// ```
    pub fn parse(text: &'a str) -> anyhow::Result<Self> {
        // first split text by whitespace, then iterate over it
        let mut iter = text.split_whitespace();
        match iter.next() {
            Some("team") => match iter.next() {
                Some("create") => match iter.next() {
                    Some(team_name) => Ok(SlashAction::CreateTeam { name: team_name }),
                    None => Ok(SlashAction::ParsingFailed(
                        "Please specify a team name when creating a team".into(),
                    )),
                },
                Some("delete") => match iter.next() {
                    Some(team_name) => Ok(SlashAction::DeleteTeam { name: team_name }),
                    None => Ok(SlashAction::ParsingFailed(
                        "Please specify a team name to delete".into(),
                    )),
                },

                Some("list") => Ok(SlashAction::ListTeams),

                Some(team_name) => match iter.next() {
                    Some("add") => match iter.next() {
                        Some(user) => Ok(SlashAction::AddMember {
                            team: team_name,
                            user,
                        }),
                        None => Ok(SlashAction::ParsingFailed(
                            format!("Please specify a user to add to team {}", team_name).into(),
                        )),
                    },
                    Some("del") => match iter.next() {
                        Some(user) => Ok(SlashAction::RemoveMember {
                            team: team_name,
                            user,
                        }),
                        None => Ok(SlashAction::ParsingFailed(
                            format!("Please specify a user to delete from team {}", team_name)
                                .into(),
                        )),
                    },
                    _ => Ok(SlashAction::ParsingFailed(
                        "Please specify either the `add` or `del` command".into(),
                    )),
                },
                _ => Ok(SlashAction::ParsingFailed(
                    "Please specify `create`, `delete`, or a team name".into(),
                )),
            },
            Some(user) if user.starts_with(|c| c == '<' || c == '@') => {
                Ok(SlashAction::ShowUser { user })
            }
            Some(team) => Ok(SlashAction::ShowTeam { team }),
            None => Ok(SlashAction::ParsingFailed(
                "Please specify a username, team name, or `team`".into(),
            )),
        }
    }
}

/// Handle a `POST` request to the `/location` endpoint
///
/// # Arguments
/// * `req` - Incoming HTTP request
pub async fn location(mut req: tide::Request<State>) -> tide::Result<tide::Response> {
    // parse the encoded form into a slash command, extracting the relevant details
    let form: SlashCommand = match req.body_form().await {
        Ok(form) => form,
        Err(e) => {
            tracing::error!("Failed to parse location request: {:?}", e);
            return Ok(tide::Response::builder(StatusCode::Ok).build());
        }
    };

    // grab a connection to the database
    let mut db = req.db().await?;

    // create our response structure of blocks
    let mut blocks: Vec<Value> = vec![];

    // parse and execute the text received as commands
    match SlashAction::parse(&form.text)? {
        SlashAction::ShowUser { user } => match User::fetch(&mut db, user).await {
            Some(user) => match user.status {
                Some(status) => mrkdwn!(blocks, format!("*<@{}>*: {}", user.id, status)),
                None => mrkdwn!(blocks, format!("*<@{}>* has not set a status", user.id)),
            },
            None => mrkdwn!(blocks, "User not found"),
        },

        SlashAction::ShowTeam { team } => match Team::members(&mut db, team).await {
            Ok(members) => {
                header!(blocks, format!("{} Status", team));
                divider!(blocks);
                for member in members {
                    match member.status {
                        Some(status) => mrkdwn!(blocks, format!("*<@{}>*: {}", member.id, status)),
                        None => mrkdwn!(blocks, format!("*<@{}>* has not set a status", member.id)),
                    }
                }
            }
            Err(_) => mrkdwn!(blocks, format!("Team *{}* not found", team)),
        },

        SlashAction::ListTeams => match Team::fetch_all(&mut db).await {
            Ok(teams) => {
                header!(blocks, "Available Teams:");
                divider!(blocks);
                for team in teams {
                    mrkdwn!(blocks, format!("• {}", team.name));
                }
            }
            Err(_) => mrkdwn!(blocks, "Failed to fetch teams"),
        },

        SlashAction::CreateTeam { name } => match Team::new(&mut db, name).await {
            Ok(team) => mrkdwn!(
                blocks,
                format!("Team *{}* successfully created!", team.name)
            ),
            Err(_) => mrkdwn!(
                blocks,
                format!("Failed to create Team {}, perhaps it already exists?", name)
            ),
        },

        SlashAction::DeleteTeam { name } => match Team::fetch(&mut db, name).await {
            Some(team) => match team.delete(&mut db).await {
                Ok(_) => mrkdwn!(blocks, format!("Team *{}* deleted", name)),
                Err(_) => mrkdwn!(
                    blocks,
                    format!("Failed to delete Team *{}*. Please try again later", name)
                ),
            },
            None => mrkdwn!(blocks, format!("Team *{}* not found", name)),
        },

        SlashAction::AddMember { team, user } => match Team::fetch(&mut db, team).await {
            Some(team) => match User::fetch_or_create(&mut db, user).await {
                Ok(user) => match team.add_member(&mut db, &user).await {
                    Ok(_) => mrkdwn!(
                        blocks,
                        format!("<@{}> added to team {}", user.id, team.name)
                    ),
                    Err(_) => mrkdwn!(
                        blocks,
                        format!("Failed to add user <@{}> to Team {}", user.id, team.name)
                    ),
                },
                Err(_) => mrkdwn!(blocks, format!("Failed to load user with id <@{}>", user)),
            },
            None => mrkdwn!(blocks, format!("Team *{}* not found", team)),
        },

        SlashAction::RemoveMember { team, user } => match Team::fetch(&mut db, team).await {
            Some(team) => match User::fetch(&mut db, user).await {
                Some(user) => match team.delete_member(&mut db, &user).await {
                    Ok(_) => mrkdwn!(
                        blocks,
                        format!("<@{}> deleted from team {}", user.id, team.name)
                    ),
                    Err(_) => mrkdwn!(
                        blocks,
                        format!(
                            "Failed to delete user <@{}> from Team {}",
                            user.id, team.name
                        )
                    ),
                },
                None => mrkdwn!(blocks, format!("User with id *{}* not found", user)),
            },
            None => mrkdwn!(blocks, format!("Team *{}* not found", team)),
        },

        SlashAction::ParsingFailed(reason) => {
            mrkdwn!(blocks, "*Oh-no!* Invalid command or arguments");
            divider!(blocks);
            mrkdwn!(blocks, reason);
        }
    }

    Ok(tide::Response::builder(StatusCode::Ok)
        .header("Content-Type", "application/json")
        .body(json!({ "blocks": blocks }))
        .build())
}
