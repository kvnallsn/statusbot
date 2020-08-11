mod handlers {
    pub(crate) mod command;
    pub(crate) mod event;
    pub(crate) mod register;
}

use anyhow::Result;
use async_std::task;
use parking_lot::RwLock;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tide::{
    http::headers::HeaderValue,
    security::{CorsMiddleware, Origin},
    StatusCode,
};
use tide_tracing::TraceMiddleware;
use tracing::Level;

#[derive(Clone, Debug)]
pub struct State {
    /// mapping of team name to a list of team members
    teams: Arc<RwLock<HashMap<String, Vec<String>>>>,

    /// Current status of individual members
    status_map: Arc<RwLock<HashMap<String, String>>>,
}

impl State {
    pub fn new() -> Self {
        State {
            teams: Arc::new(RwLock::new(HashMap::new())),
            status_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

pub async fn handle_post(mut req: tide::Request<State>) -> tide::Result<tide::Response> {
    // first decode the body as an unknown JSON request to extract the type
    let body = req.body_bytes().await?;
    let json: Value = serde_json::from_slice(&body)?;

    match json["type"].as_str() {
        Some("url_verification") => handlers::register::url_verification(&body),
        Some("event_callback") => handlers::event::callback(&body, req.state()).await,

        // ignore all other events, but respond with 200 OK so we don't get blocked by Slack
        _ => Ok(tide::Response::builder(StatusCode::Ok).build()),
    }
}

async fn run_server() -> Result<()> {
    // configure logging via `Tracing`
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // configure CORS middleware
    let cors = CorsMiddleware::new()
        .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"))
        .allow_credentials(false);

    // configure tracing middleware
    let trace = TraceMiddleware::new();

    // create the actual web app
    let mut app = tide::with_state(State::new());

    // enable middlewares
    app.with(cors);
    app.with(trace);

    // add routes
    //app.at("/").post(routes::register::post_register);
    app.at("/").post(handle_post);
    app.at("/location").post(handlers::command::location);

    // run the app
    app.listen("0.0.0.0:5010").await?;

    Ok(())
}

fn main() {
    task::block_on(async {
        if let Err(e) = run_server().await {
            eprintln!("{:?}", e);
        }
    });
}
