mod handlers {
    pub(crate) mod command;
    pub(crate) mod event;
    pub(crate) mod register;
}

mod models {
    mod team;
    mod user;

    pub use self::team::Team;
    pub use self::user::User;
}

use anyhow::Result;
use async_std::task;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::{pool::PoolConnection, sqlite::SqlitePool, Sqlite};
use tide::{
    http::headers::HeaderValue,
    security::{CorsMiddleware, Origin},
    StatusCode,
};
use tide_tracing::TraceMiddleware;
use tracing::Level;

type SqlConn = PoolConnection<Sqlite>;

#[async_trait]
pub trait HasDb {
    type Target;
    type Error;

    async fn db(&self) -> std::result::Result<Self::Target, Self::Error>;
}

#[async_trait]
impl HasDb for tide::Request<State> {
    type Target = PoolConnection<Sqlite>;
    type Error = sqlx::Error;

    async fn db(&self) -> std::result::Result<Self::Target, Self::Error> {
        self.state().pool.acquire().await
    }
}

#[derive(Clone, Debug)]
pub struct State {
    /// A configured sqlite pool
    pool: SqlitePool,
}

impl State {
    pub fn new(pool: SqlitePool) -> Self {
        State { pool }
    }
}

/// Handles all `POST`s received to the root (`/`) uri.
///
/// Depending on the `type` JSON field, dispatches messages to the appropriate handler
///
/// # Arguments
/// * `req`- Incoming HTTP request
pub async fn handle_post(mut req: tide::Request<State>) -> tide::Result<tide::Response> {
    // first decode the body as an unknown JSON request to extract the type
    let body = req.body_bytes().await?;
    let json: Value = serde_json::from_slice(&body)?;

    // now get a connection to the sqlite database
    let mut conn = req.db().await?;

    match json["type"].as_str() {
        Some("url_verification") => handlers::register::url_verification(&body),
        Some("event_callback") => handlers::event::callback(&body, &mut conn).await,

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

    // connect to sqlite and build connection pool
    let pool = SqlitePool::connect("sqlite://statusbot.sqlite3").await?;

    // create the actual web app
    let mut app = tide::with_state(State::new(pool));

    // enable middlewares
    app.with(cors);
    app.with(trace);

    // add routes
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
