use crate::State;
use serde::Deserialize;
use serde_json::{json, Value};
use tide::StatusCode;

macro_rules! extract_user_id {
    ($user:expr) => {
        $user
            .trim_matches(|c| c == '<' || c == '>' || c == '@')
            .split('|')
            .next()
    };
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

pub async fn location(mut req: tide::Request<State>) -> tide::Result<tide::Response> {
    let form: SlashCommand = match req.body_form().await {
        Ok(form) => form,
        Err(e) => {
            tracing::error!("Failed to parse location request: {:?}", e);
            return Ok(tide::Response::builder(StatusCode::Ok).build());
        }
    };

    // split text by spaces
    let mut text = form.text.split_whitespace();
    let cmd = match text.next() {
        Some(c) => c,
        None => {
            tracing::error!("No command entered");
            return Ok(tide::Response::builder(StatusCode::Ok).build());
        }
    };

    let blocks = match cmd {
        "" => location_help(),
        "team" => location_team_cmd(text, req.state()),
        user if user.starts_with(|c| c == '<' || c == '@') => location_user_cmd(user, req.state()),
        team => location_team_status(team, req.state()),
    };

    let body = json!({ "blocks": blocks });
    //tracing::debug!("{:#?}", body);

    Ok(tide::Response::builder(StatusCode::Ok)
        .header("Content-Type", "application/json")
        .body(json!({ "blocks": blocks }))
        .body(body)
        .build())
}

/// Returns a simple help message about what commands are supported
pub fn location_help() -> Vec<Value> {
    vec![json!({})]
}

#[derive(Clone, Debug)]
pub enum TeamMemberAction {
    Add,
    Delete,
}

/// Process the teams command
pub fn location_team_cmd(mut iter: std::str::SplitWhitespace, state: &State) -> Vec<Value> {
    let mut blocks = vec![];

    // get the team name or the command
    match iter.next() {
        Some("create") => match iter.next() {
            Some(team_name) => {
                let mut teams = state.teams.write();
                teams.insert(team_name.to_owned(), vec![]);

                blocks.push(json!({
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!("Team '{}' successfully created", team_name),
                    }
                }));
            }
            None => blocks.push(json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": "Please supply a team name when creating a team",
                }
            })),
        },
        Some("delete") => match iter.next() {
            Some(team_name) => {
                let mut teams = state.teams.write();
                match teams.remove(team_name) {
                    Some(_) => blocks.push(json!({
                        "type": "section",
                        "text": {
                            "type": "mrkdwn",
                            "text": format!("Team '{}' successfully deleted", team_name),
                        }
                    })),
                    None => blocks.push(json!({
                        "type": "section",
                        "text": {
                            "type": "mrkdwn",
                            "text": format!("Team '{}' not found", team_name),
                        }
                    })),
                }
            }
            None => blocks.push(json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": "Please supply a team name when deleting a team",
                }
            })),
        },

        Some(team) => match iter.next() {
            Some("add") => blocks.append(&mut location_team_add_del_user(
                team,
                TeamMemberAction::Add,
                iter,
                state,
            )),
            Some("del") => blocks.append(&mut location_team_add_del_user(
                team,
                TeamMemberAction::Delete,
                iter,
                state,
            )),
            None => blocks.append(&mut location_team_list_members(team, state)),
            _ => blocks.push(json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("Team '{}' not found", team)
                }
            })),
        },
        None => blocks.push(json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": "Please supply a team name or command",
            }
        })),
    };

    blocks
}

/// Adds a user to a specific team
///
/// # Arguments
/// * `team` - Team to interact with
/// * `action` - Action to execute (add, remove)
/// * `iter` - iterator containg rest of string to process
/// * `state` - Application state
pub fn location_team_add_del_user(
    team: &str,
    action: TeamMemberAction,
    mut iter: std::str::SplitWhitespace,
    state: &State,
) -> Vec<Value> {
    let mut blocks = vec![];

    match iter.next() {
        Some(user) => match extract_user_id!(user) {
            Some(user) => {
                let mut teams = state.teams.write();
                match teams.get_mut(team) {
                    Some(ref mut team_vec) => {
                        match action {
                            TeamMemberAction::Add => team_vec.push(user.to_owned()),
                            TeamMemberAction::Delete => (),
                        }
                        blocks.push(json!({
                            "type": "section",
                            "text": {
                                "type": "mrkdwn",
                                "text": format!("Successfully {} '<@{}>' to team '{}'", match action { TeamMemberAction::Add => "added", TeamMemberAction::Delete => "deleted" }, user, team),
                            }
                        }));
                    }
                    None => blocks.push(json!({
                        "type": "section",
                        "text": {
                            "type": "mrkdwn",
                            "text": format!("Team '{}' does not exist", team),
                        }
                    })),
                }
            }
            None => blocks.push(json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("Please supply a user to {} to a team", match action { TeamMemberAction::Add => "add", TeamMemberAction::Delete => "delete" }),
                }
            })),
        },
        None => blocks.push(json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": "Could not parse username or id",
            }
        }))
    }

    blocks
}

pub fn location_team_list_members(team: &str, state: &State) -> Vec<Value> {
    let mut blocks = vec![];

    let teams = state.teams.read();
    if let Some(members) = teams.get(team) {
        blocks.push(json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": format!("*{} members:*", team)
            }
        }));

        for member in members {
            blocks.push(json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("<@{}>", member)
                }
            }));
        }
    }

    blocks
}

/// Handle printing status for a user
pub fn location_user_cmd(user: &str, state: &State) -> Vec<Value> {
    let mut blocks = vec![];
    let status_map = state.status_map.read();

    match extract_user_id!(user) {
        Some(user) => match status_map.get(user) {
            Some(status) => blocks.push(json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("<@{}>: {}", user, status),
                }
            })),

            // if we didn't find a user, it could be a team name
            None => blocks.push(json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("<@{}> has not set a status", user),
                }
            })),
        },
        None => blocks.push(json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": format!("<@{}> not found", user),
            }
        })),
    }

    blocks
}

/// Handle printing status for a user
pub fn location_team_status(team: &str, state: &State) -> Vec<Value> {
    let mut blocks = vec![];

    let teams = state.teams.read();
    if let Some(members) = teams.get(team) {
        blocks.push(json!({
            "type": "header",
            "text": {
                "type": "mrkdwn",
                "text": format!("{} Status", team),
            }
        }));

        blocks.push(json!({ "type": "divider" }));

        for member in members {
            blocks.append(&mut location_user_cmd(member, state));
        }
    } else {
        blocks.push(json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": format!("Team {} not found", team),
            }
        }));
    }

    blocks
}
