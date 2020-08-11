//! Register this slack app

use crate::State;
use anyhow::Result;
use dotenv_codegen::dotenv;
use serde::Deserialize;
use serde_json::json;
use tide::StatusCode;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AppEvent {
    #[serde(alias = "app_mention")]
    AppMention {
        user: String,
        text: String,
        ts: String,
        channel: String,
        event_ts: String,
    },

    #[serde(alias = "message")]
    Message {
        channel: String,
        user: String,
        text: String,
        ts: String,
        event_ts: String,
        channel_type: String,
    },
}

/// Structure received via `POST` request for registering a form
#[derive(Debug, Deserialize)]
struct Event {
    /// This depcrecated verification token is proof the request is coming from Slack
    pub token: String,

    /// Unique team id that generated the event
    pub team_id: String,

    /// API App Id (as seen in App Home)
    pub api_app_id: String,

    /// Type of request received (e.g. "url_verification")
    #[serde(alias = "type")]
    pub ty: String,

    /// Specific event details
    pub event: AppEvent,

    /// The authorized users involved in the event
    pub authed_users: Vec<String>,

    /// Unique id of this event
    pub event_id: String,

    /// Timestamp this event occured
    pub event_time: u64,
}

pub async fn callback(body: &[u8], state: &State) -> tide::Result<tide::Response> {
    // deserialize into the actual event type
    let event: Event = match serde_json::from_slice(body) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Callback parse error: {:?}", e);

            // if parsing fails, just respond with `200 OK` else slack will ban our bot eventually
            return Ok(tide::Response::builder(StatusCode::Ok).build());
        }
    };

    handle_app_event(event.event, state).await?;

    let resp = tide::Response::builder(StatusCode::Ok).build();

    Ok(resp)
}

pub async fn handle_app_event(app_event: AppEvent, state: &State) -> Result<()> {
    match app_event {
        AppEvent::AppMention {
            user,
            text,
            channel,
            ..
        } => handle_mention(state, user, text, channel).await,

        AppEvent::Message {
            user,
            text,
            channel,
            ..
        } => handle_message(state, user, text, channel).await,
    }
}

/// Handles an `app_mention` event
///
/// # Arguments
/// * `state` - Application State
/// * `user` - User who mentioned the bot
/// * `text` - Text the user entered
/// * `channel` - What channel this occured in
pub async fn handle_mention(
    state: &State,
    user: String,
    text: String,
    channel: String,
) -> Result<()> {
    {
        // first lock the mutex to perform a write update
        // parse text contained as the user's status
        let mut map = state.status_map.write();
        map.insert(user.clone(), text.clone());
    }

    let resp = surf::post("https://slack.com/api/chat.postEphemeral")
        .set_header(
            "Authorization",
            format!("Bearer {}", dotenv!("SLACK_BOT_TOKEN")),
        )
        .body_json(&json!({
            "text": format!("Hello <@{}> who said '{}'", user, text),
            "user": user,
            "channel": channel,
        }))?
        .await
        .unwrap();

    let code = resp.status();
    if code.is_client_error() || code.is_server_error() {
        tracing::error!("Failed to post ephemeral message: {}", resp.status());
    }

    Ok(())
}

/// Handles an `app_mention` event
///
/// # Arguments
/// * `state` - Application State
/// * `user` - User who mentioned the bot
/// * `text` - Text the user entered
/// * `channel` - What channel this occured in
pub async fn handle_message(
    state: &State,
    user: String,
    text: String,
    channel: String,
) -> Result<()> {
    // TODO verify the channel is daily_status
    {
        // first lock the mutex to perform a write update
        // parse text contained as the user's status
        let mut map = state.status_map.write();
        map.insert(user.clone(), text.clone());
    }

    Ok(())
}
