//! Register this slack app

use serde::Deserialize;
use serde_json::json;
use tide::StatusCode;

/// Structure received via `POST` request for registering a form
#[derive(Debug, Deserialize)]
struct FormRegister {
    /// This depcrecated verification token is proof the request is coming from Slack
    pub token: String,

    /// Value to respond with, completing the registration challenge
    pub challenge: String,

    /// Type of request received (e.g. "url_verification")
    #[serde(alias = "type")]
    pub ty: String,
}

/// Handles initial registration of bot with Slack
///
/// # Arguments
/// * `body` - Request body to parse as JSON
pub fn url_verification(body: &[u8]) -> tide::Result<tide::Response> {
    let form: FormRegister = serde_json::from_slice(body)?;

    match dotenv::var("SLACK_APP_TOKEN") {
        Ok(token) if token == form.token => {
            let resp = tide::Response::builder(StatusCode::Ok)
                .body(json!({ "challenge": form.challenge }))
                .build();

            Ok(resp)
        }
        _ => Ok(tide::Response::builder(StatusCode::BadRequest).build()),
    }
}
