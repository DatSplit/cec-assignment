use clap::Parser;
use dotenv;
use metric::Metrics;
use poem::{listener::TcpListener, EndpointExt, Route};
use poem_openapi::OpenApiService;
use prometheus_client::registry::Registry;
use sqlx::postgres::PgPoolOptions;
use std::{
    env,
    sync::{Arc, Mutex},
};
use tracing::{info, Level};

mod api;
mod jwt;
mod metric;
mod store;

use api::{Api, SecretKey};

#[derive(Parser, Debug)]
struct CliArgs {
    #[arg(short, long)]
    secret_key: String,

    #[arg(short, long)]
    external_ip: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    dotenv::from_filename("notifications-service/.env")?;

    let args = CliArgs::parse();
    let secret_key = SecretKey(args.secret_key);

    let metrics = Metrics::new();
    let mut registry = <Registry>::default();
    registry.register(
        "response_count",
        "Count of response",
        metrics.response_count.clone(),
    );
    let state = Arc::new(Mutex::new(registry));

    let pool = match env::var("DATABASE_URL") {
        Ok(database_url) => {
            let pool = Some(
                PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&database_url)
                    .await
                    .expect("Unable to connect to database provided in DATABASE_URL"),
            );
            info!("Created connection pool to database");
            pool
        }
        _ => None,
    };

    let external_ip = args.external_ip;
    let api_service = OpenApiService::new(Api, "Notifications Service", "1.1")
        .server(format!("http://{}:3000/api", external_ip));

    let ui = api_service.swagger_ui();
    let app = Route::new()
        .nest("/api", api_service)
        .nest("/", ui)
        .data(secret_key)
        .data(state)
        .data(metrics.clone())
        .data(pool);

    Ok(poem::Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await?)
}
