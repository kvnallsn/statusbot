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
use sqlx::pool::PoolConnection;
use std::fmt;
use structopt::StructOpt;
use tide::{
    http::headers::HeaderValue,
    security::{CorsMiddleware, Origin},
    StatusCode,
};
use tide_tracing::TraceMiddleware;
use tracing::Level;

#[cfg(all(feature = "sqlite", feature = "postgres"))]
compile_error!("Must enable only feature `sqlite` or `postgres`. Bot cannot be enabled");

#[cfg(not(any(feature = "sqlite", feature = "postgres")))]
compile_error!("Must enable either feature `sqlite` or `postgres`. Bot cannot be enabled");

#[cfg(feature = "sqlite")]
type SqlPool = sqlx::sqlite::SqlitePool;
#[cfg(feature = "sqlite")]
type SqlConn = PoolConnection<sqlx::Sqlite>;

#[cfg(feature = "postgres")]
type SqlPool = sqlx::postgres::PgPool;
#[cfg(feature = "postgres")]
type SqlConn = PoolConnection<sqlx::Postgres>;

/// Command line options and arguments
#[derive(StructOpt, Debug)]
#[structopt(name = "statusbot")]
struct Opt {
    /// Database connection string
    // SQLite: `sqlite://statusbot.sqlite3`
    // Postgres: `postgres://<username>:<password>@<host>:<port>/<database>`
    #[structopt(
        short,
        long,
        env = "DATABASE_URL",
        default_value = "sqlite://statusbot.sqlite3"
    )]
    database: String,

    /// IP address to listen on/bind
    #[structopt(short, long, default_value = "0.0.0.0")]
    host: String,

    /// Port to listen on/bind
    #[structopt(short, long, default_value = "5010")]
    port: u16,

    /// Skip running migrations when app starts
    #[structopt(long)]
    skip_migrations: bool,
}

impl fmt::Display for Opt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "host={}, port={}", self.host, self.port)
    }
}

#[async_trait]
pub trait HasDb {
    //type Target;
    type Error;

    async fn db(&self) -> std::result::Result<SqlConn, Self::Error>;
}

#[async_trait]
impl HasDb for tide::Request<State> {
    //type Target = SqlConn;
    type Error = sqlx::Error;

    async fn db(&self) -> std::result::Result<SqlConn, Self::Error> {
        self.state().pool.acquire().await
    }
}

#[derive(Clone, Debug)]
pub struct State {
    /// A configured sql pool
    pool: SqlPool,
}

impl State {
    pub fn new(pool: SqlPool) -> Self {
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

    // now get a connection to the sql database
    let mut conn: SqlConn = req.db().await?;

    match json["type"].as_str() {
        Some("url_verification") => handlers::register::url_verification(&body),
        Some("event_callback") => handlers::event::callback(&body, &mut conn).await,

        // ignore all other events, but respond with 200 OK so we don't get blocked by Slack
        _ => Ok(tide::Response::builder(StatusCode::Ok).build()),
    }
}

async fn run_migrations(db: &SqlPool) -> Result<()> {
    use sqlx::migrate::Migrator;
    use std::path::Path;

    #[cfg(feature = "postgres")]
    let path = Path::new("./postgres/migrations");

    #[cfg(feature = "sqlite")]
    let path = Path::new("./sqlite/migrations");

    tracing::info!("running migrations [{}]", path.display());

    let migrator = Migrator::new(path).await?;
    match migrator.run(db).await {
        Ok(()) => tracing::info!("migrations complete"),
        Err(e) => {
            tracing::error!("failed to run migrations:\n{:?}", e);
        }
    }

    Ok(())
}

async fn run_server(opt: Opt) -> Result<()> {
    // configure CORS middleware
    let cors = CorsMiddleware::new()
        .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"))
        .allow_credentials(false);

    // configure tracing middleware
    let trace = TraceMiddleware::new();

    // connect to sql and build connection pool
    let pool = SqlPool::connect(&opt.database).await?;

    if !opt.skip_migrations {
        // run migrations
        run_migrations(&pool).await?;
    }

    // create the actual web app
    let mut app = tide::with_state(State::new(pool));

    // enable middlewares
    app.with(cors);
    app.with(trace);

    // add routes
    app.at("/").post(handle_post);
    app.at("/location").post(handlers::command::location);

    // run the app
    tracing::info!("Starting web server");
    app.listen(format!("{}:{}", opt.host, opt.port)).await?;

    Ok(())
}

fn main() -> Result<()> {
    // load environment variables from .env file
    dotenv::dotenv().ok();

    let opt = Opt::from_args();

    // configure logging via `Tracing`
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    tracing::info!("Starting StatusBot");
    tracing::debug!("ARGS {}", opt);

    task::block_on(async {
        if let Err(e) = run_server(opt).await {
            eprintln!("Failed to run server: {:?}", e);
        }
    });

    Ok(())
}
