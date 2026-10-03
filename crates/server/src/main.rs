use dashmap::DashMap;
use hexabellum_server::{build_router, MatchRegistry};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hexabellum_server=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let registry: MatchRegistry = Arc::new(DashMap::new());
    let app = build_router(registry);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Hexabellum Authoritative Server listening on {}", addr);

    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            if let Err(err) = axum::serve(listener, app).await {
                tracing::error!("Server error: {:?}", err);
            }
        }
        Err(err) => {
            tracing::error!("Failed to bind to {}: {:?}", addr, err);
            std::process::exit(1);
        }
    }
}
