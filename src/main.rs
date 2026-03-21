mod app;
mod config;
mod store;

use std::sync::Arc;

use anyhow::Result;
use app::{build_router, AppState};
use config::AppConfig;
use store::{PgStore, StateStore};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "tf_http_pg_backend=info,info".to_string()),
        )
        .init();

    let config = AppConfig::from_env()?;
    let store = PgStore::connect(&config.database_url, config.max_db_connections).await?;
    store.migrate().await?;

    let app_state = AppState {
        store: Arc::new(store) as Arc<dyn StateStore>,
        basic_auth: config.basic_auth,
    };
    let app = build_router(app_state);

    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;
    info!("listening on {}", config.listen_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
