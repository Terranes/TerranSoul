//! TerranSoul Hive Relay — standalone gRPC server binary.
//!
//! Usage:
//!   hive-relay --database-url postgres://... --listen 0.0.0.0:50051

use std::sync::Arc;

use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use tonic::transport::Server;
use tracing_subscriber::EnvFilter;

use hive_relay::db::RelayDb;
use hive_relay::proto::hive_relay_server::HiveRelayServer;
use hive_relay::relay::HiveRelayService;

#[derive(Parser)]
#[command(name = "hive-relay", version, about = "TerranSoul Hive relay server")]
struct Cli {
    /// PostgreSQL connection URL.
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// Listen address for the gRPC server.
    #[arg(long, default_value = "0.0.0.0:50051", env = "LISTEN_ADDR")]
    listen: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env if present
    let _ = dotenvy::dotenv();

    // Initialise tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("hive_relay=info".parse()?))
        .init();

    let cli = Cli::parse();

    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&cli.database_url)
        .await?;

    let db = RelayDb::new(pool);
    db.migrate().await?;
    tracing::info!("Database schema migrated.");

    let service = HiveRelayService::new(db);
    let service = Arc::new(service);

    let addr = cli.listen.parse()?;
    tracing::info!("Hive relay listening on {addr}");

    Server::builder()
        .add_service(HiveRelayServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
