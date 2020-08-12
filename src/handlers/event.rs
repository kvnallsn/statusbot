//! Handle callback events

use crate::{models::User, SqlConn};
use anyhow::Result;
use dotenv_codegen::dotenv;
use serde::Deserialize;
use serde_json::json;
use tide::StatusCode;

/// Specific types of events that our bot is registered to receive
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AppEvent {
    /// This event occurs when somebody mentions our bot (@statusbot)
    #[serde(alias = "app_mention")]
    AppMention {
        user: String,
        text: String,
        ts: String,
        channel: String,
        event_ts: String,
    },

    /// This event occurs when any messages that our bot has been invited to occur.  Examples of
    /// messages occuring are posting new messages, deleting messages, etc.
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

/// Handle the event callback from a `POST` request
///
/// # Arguments
/// * `body` - The body of the POST request
/// * `db` - Conenction to the sql database
pub async fn callback(body: &[u8], db: &mut SqlConn) -> tide::Result<tide::Response> {
    // deserialize into the actual event type
    let event: Event = match serde_json::from_slice(body) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Callback parse error: {:?}", e);

            // if parsing fails, just respond with `200 OK` else slack will ban our bot eventually
            return Ok(tide::Response::builder(StatusCode::Ok).build());
        }
    };

    handle_app_event(event.event, db).await?;

    let resp = tide::Response::builder(StatusCode::Ok).build();

    Ok(resp)
}

/// Handle the actual event received after it has been unpacked
///
/// # Arguments
/// * `app_event` - Specific event received
/// * `db` - Connection to the SQL database
pub async fn handle_app_event(app_event: AppEvent, db: &mut SqlConn) -> Result<()> {
    match app_event {
        AppEvent::AppMention {
            user,
            text,
            channel,
            event_ts,
            ..
        } => handle_mention(db, user, text, channel, event_ts).await,

        AppEvent::Message {
            user,
            text,
            channel,
            ..
        } => handle_message(db, user, text, channel).await,
    }
}

/// Handles an `app_mention` event
///
/// # Arguments
/// * `user` - User who mentioned the bot
/// * `text` - Text the user entered
/// * `channel` - What channel this occured in
/// * `event_ts` - The timestamp the event occured (used in response to add emoji)
pub async fn handle_mention(
    db: &mut SqlConn,
    user: String,
    text: String,
    channel: String,
    event_ts: String,
) -> Result<()> {
    // strip statusbot prefix, but if striping fails, keep the original text
    let status = text
        .strip_prefix("@statusbot ")
        .map(|s| s.to_owned())
        .unwrap_or_else(|| text);

    let mut user = User::new(user);
    user.set_status(status);
    user.save(&mut *db).await?;

    // Respond with a thumbs up to let the user know the message has been received
    let resp = surf::post("https://slack.com/api/reactions.add")
        .set_header(
            "Authorization",
            format!("Bearer {}", dotenv!("SLACK_BOT_TOKEN")),
        )
        .body_json(&json!({
            "channel": channel,
            "name": "thumbsup",
            "timestamp": event_ts
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
/// * `user` - User who mentioned the bot
/// * `text` - Text the user entered
/// * `channel` - What channel this occured in
pub async fn handle_message(
    db: &mut SqlConn,
    user: String,
    text: String,
    _channel: String,
) -> Result<()> {
    // TODO verify the channel is daily_status

    let mut user = User::new(user);
    user.set_status(text);
    user.save(&mut *db).await?;

    // Note: since this is a passive monitor, we don't acknowledge receiving the messages

    Ok(())
}
