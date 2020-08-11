use crate::State;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
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

    let status_map = req.state().status_map.read();

    // split text by spaces
    let mut text = form.text.split_whitespace();
    let cmd = match text.next() {
        Some(c) => c,
        None => {
            tracing::error!("No command entered");
            return Ok(tide::Response::builder(StatusCode::Ok).build());
        }
    };

    let body = match cmd {
        "" => json!({}),
        "office" => json!({}),
        "telework" => json!({}),
        "team" => location_team_cmd(text),
        user => location_user_cmd(user, &status_map),
    };

    Ok(tide::Response::builder(StatusCode::Ok)
        .header("Content-Type", "application/json")
        .body(body)
        .build())
}

pub fn location_team_cmd(mut iter: std::str::SplitWhitespace) -> Value {
    let team = match iter.next() {
        Some(team) => team,
        None => {
            return json!({
                    "blocks": [
                        {
                            "type": "section",
                            "text": {
                                "type": "mrkdwn",
                                "text": "Please supply a team name or command",
                            }
                        }
                    ]
            });
        }
    };

    match team {
        "create" => (), // create a new team
        "delete" => (), // delete a team
        _ => (),        // team sub-sub commands
    }

    if let Some(subcmd) = iter.next() {
        match subcmd {
            "add" => (),
            "del" => (),
            "" => (), // print status
            _ => (),
        }
    }

    json!({})
}

pub fn location_user_cmd(user: &str, status_map: &HashMap<String, String>) -> Value {
    match extract_user_id!(user) {
        Some(user) => match status_map.get(user) {
            Some(status) => json!({
                    "blocks": [
                        {
                            "type": "section",
                            "text": {
                                "type": "mrkdwn",
                                "text": format!("<@{}>: {}", user, status),
                            }
                        }
                    ]
            }),
            None => json!({
                    "blocks": [
                        {
                            "type": "section",
                            "text": {
                                "type": "mrkdwn",
                                "text": format!("<@{}> has not set a status", user),
                            }
                        }
                    ]
            }),
        },
        None => json!({
                    "blocks": [
                        {
                            "type": "section",
                            "text": {
                                "type": "mrkdwn",
                                "text": format!("<@{}> not found", user),
                            }
                        }
                    ]
        }),
    }
}
